use super::{
    corsa_session::{CorsaTypeAwareSession, TypeProbe},
    LintResult, Linter,
};
use crate::diagnostic::LintDiagnostic;
use corsa::utils::{
    is_any_like_type_texts, is_promise_like_type_texts, is_unknown_like_type_texts,
};
use oxc_allocator::Allocator as OxcAllocator;
use oxc_ast::ast::{
    CallExpression, ChainElement, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey,
    Statement,
};
use oxc_parser::Parser as OxcParser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::operator::UnaryOperator;
use std::path::Path;
use vize_armature::Parser as TemplateParser;
use vize_carton::{String, ToCompactString};
use vize_croquis::{
    virtual_ts::{generate_virtual_ts, VirtualTsOutput},
    Analyzer, AnalyzerOptions,
};

const RULE_REQUIRE_TYPED_PROPS: &str = "type/require-typed-props";
const RULE_REQUIRE_TYPED_EMITS: &str = "type/require-typed-emits";
const RULE_NO_FLOATING_PROMISES: &str = "type/no-floating-promises";

pub(crate) fn has_active_type_aware_rules(linter: &Linter) -> bool {
    [
        RULE_REQUIRE_TYPED_PROPS,
        RULE_REQUIRE_TYPED_EMITS,
        RULE_NO_FLOATING_PROMISES,
    ]
    .into_iter()
    .any(|rule_name| linter.registry.has_rule(rule_name) && linter.is_rule_enabled(rule_name))
}

pub(crate) fn lint_sfc_with_corsa(linter: &Linter, source: &str, filename: &str) -> LintResult {
    let mut result = match super::script_rules::parse_sfc_for_lint(source, filename) {
        Ok(descriptor) => lint_with_descriptor(linter, source, filename, descriptor),
        Err(_) => {
            if let Some((content, byte_offset)) = super::engine::extract_template_fast(source) {
                let mut fallback = linter.lint_template(&content, filename);
                if byte_offset > 0 {
                    for diag in &mut fallback.diagnostics {
                        diag.start += byte_offset;
                        diag.end += byte_offset;
                        for label in &mut diag.labels {
                            label.start += byte_offset;
                            label.end += byte_offset;
                        }
                    }
                }
                fallback
            } else {
                LintResult {
                    filename: filename.to_compact_string(),
                    diagnostics: Vec::new(),
                    error_count: 0,
                    warning_count: 0,
                }
            }
        }
    };

    result.filename = filename.to_compact_string();
    result
}

fn lint_with_descriptor<'a>(
    linter: &Linter,
    source: &str,
    filename: &str,
    descriptor: vize_atelier_sfc::SfcDescriptor<'a>,
) -> LintResult {
    let mut result = super::script_rules::lint_with_descriptor(linter, filename, &descriptor);

    let Some(script_block) = descriptor
        .script_setup
        .as_ref()
        .or(descriptor.script.as_ref())
    else {
        return result;
    };

    let script_content = script_block.content.as_ref();
    if script_content.is_empty() {
        return result;
    }

    let allocator =
        vize_carton::Allocator::with_capacity((source.len() * 4).max(linter.initial_capacity));
    let template_ast = descriptor.template.as_ref().map(|template| {
        let parser = TemplateParser::new(allocator.as_bump(), &template.content);
        let (root, _) = parser.parse();
        (root, template.loc.start as u32)
    });

    let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
    if let Some(script_setup) = descriptor.script_setup.as_ref() {
        let generic = script_setup
            .attrs
            .get("generic")
            .map(|value| value.as_ref());
        analyzer.analyze_script_setup_with_generic(script_setup.content.as_ref(), generic);
    } else if let Some(script) = descriptor.script.as_ref() {
        analyzer.analyze_script_plain(script.content.as_ref());
    }
    if let Some((template_ast, _)) = template_ast.as_ref() {
        analyzer.analyze_template(template_ast);
    }
    let analysis = analyzer.finish();

    let template_offset = template_ast
        .as_ref()
        .map(|(_, offset)| *offset)
        .unwrap_or(0);

    let from_file = Path::new(filename).parent();
    let mut virtual_ts = generate_virtual_ts(
        Some(script_content),
        template_ast.as_ref().map(|(root, _)| root),
        &analysis.bindings,
        None,
        from_file,
        template_offset,
    );

    let mut macro_queries = Vec::new();

    if linter.registry.has_rule(RULE_REQUIRE_TYPED_PROPS)
        && linter.is_rule_enabled(RULE_REQUIRE_TYPED_PROPS)
    {
        if let Some(call) = analysis.macros.define_props() {
            if call.type_args.is_none() {
                if is_runtime_array_macro(call.runtime_args.as_ref().map(|args| args.as_str())) {
                    push_warning(
                        &mut result,
                        LintDiagnostic::warn(
                            RULE_REQUIRE_TYPED_PROPS,
                            "Prop should have a type definition",
                            script_block.loc.start as u32 + call.start,
                            script_block.loc.start as u32 + call.end,
                        )
                        .with_help(
                            "Use `defineProps<Props>()` or a runtime prop object with concrete constructor types.",
                        ),
                    );
                } else if analysis
                    .macros
                    .props()
                    .iter()
                    .any(|prop| prop.prop_type.is_none())
                {
                    push_prop_type_markers(
                        &mut virtual_ts,
                        call.runtime_args.as_ref().map(|args| args.as_str()),
                        call.start,
                        call.end,
                        &mut macro_queries,
                    );
                }
            }
        }
    }

    if linter.registry.has_rule(RULE_REQUIRE_TYPED_EMITS)
        && linter.is_rule_enabled(RULE_REQUIRE_TYPED_EMITS)
    {
        if let Some(call) = analysis.macros.define_emits() {
            if call.type_args.is_none() {
                if is_runtime_array_macro(call.runtime_args.as_ref().map(|args| args.as_str())) {
                    push_warning(
                        &mut result,
                        LintDiagnostic::warn(
                            RULE_REQUIRE_TYPED_EMITS,
                            "Emit should have a type definition",
                            script_block.loc.start as u32 + call.start,
                            script_block.loc.start as u32 + call.end,
                        )
                        .with_help(
                            "Use `defineEmits<...>()` or a validator object with typed payload parameters.",
                        ),
                    );
                } else if analysis
                    .macros
                    .emits()
                    .iter()
                    .any(|emit| emit.payload_type.is_none())
                {
                    push_emit_validator_markers(
                        &mut virtual_ts,
                        call.runtime_args.as_ref().map(|args| args.as_str()),
                        call.start,
                        call.end,
                        &mut macro_queries,
                    );
                }
            }
        }
    }

    if linter.registry.has_rule(RULE_NO_FLOATING_PROMISES)
        && linter.is_rule_enabled(RULE_NO_FLOATING_PROMISES)
    {
        for candidate in collect_floating_candidates(script_content) {
            push_promise_marker(
                &mut virtual_ts,
                script_content,
                candidate.start,
                candidate.end,
                &mut macro_queries,
            );
        }
    }

    if macro_queries.is_empty() {
        return result;
    }

    let mut should_warn_for_props = false;
    let mut should_warn_for_emits = false;

    let _ = with_corsa_session(linter, filename, |session| {
        let active_project = session.open_virtual_project(&virtual_ts.content)?;
        for query in &macro_queries {
            let probe = session.probe_type_at_offset(
                &active_project,
                &virtual_ts.content,
                query.generated_offset,
                false,
                matches!(query.kind, QueryKind::EmitValidator | QueryKind::Promise),
            )?;

            match query.kind {
                QueryKind::PropType => {
                    should_warn_for_props |= should_warn_for_prop_access(probe.as_ref());
                }
                QueryKind::EmitValidator => {
                    should_warn_for_emits |= should_warn_for_emit_validator(probe.as_ref());
                }
                QueryKind::Promise => {
                    if let Some(probe) = probe.as_ref() {
                        if has_promise_like_return(probe)
                            || is_promise_like_type_texts(&probe.type_texts, &probe.property_names)
                        {
                            push_warning(
                                &mut result,
                                LintDiagnostic::warn(
                                    RULE_NO_FLOATING_PROMISES,
                                    "Floating Promise must be awaited, returned, or explicitly ignored with `void`",
                                    script_block.loc.start as u32 + query.source_start,
                                    script_block.loc.start as u32 + query.source_end,
                                )
                                .with_help(
                                    "Add `await`, return the Promise, or prefix it with `void` when the fire-and-forget behavior is intentional.",
                                ),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    });

    if should_warn_for_props {
        if let Some(query) = macro_queries
            .iter()
            .find(|query| matches!(query.kind, QueryKind::PropType))
        {
            push_warning(
                &mut result,
                LintDiagnostic::warn(
                    RULE_REQUIRE_TYPED_PROPS,
                    "Prop should have a type definition",
                    script_block.loc.start as u32 + query.source_start,
                    script_block.loc.start as u32 + query.source_end,
                )
                .with_help(
                    "Use `defineProps<Props>()` or a runtime prop object with concrete constructor types.",
                ),
            );
        }
    }

    if should_warn_for_emits {
        if let Some(query) = macro_queries
            .iter()
            .find(|query| matches!(query.kind, QueryKind::EmitValidator))
        {
            push_warning(
                &mut result,
                LintDiagnostic::warn(
                    RULE_REQUIRE_TYPED_EMITS,
                    "Emit should have a type definition",
                    script_block.loc.start as u32 + query.source_start,
                    script_block.loc.start as u32 + query.source_end,
                )
                .with_help(
                    "Use `defineEmits<...>()` or a validator object with typed payload parameters.",
                ),
            );
        }
    }

    result
}

fn push_warning(result: &mut LintResult, diagnostic: LintDiagnostic) {
    result.warning_count += 1;
    result.diagnostics.push(diagnostic);
}

fn with_corsa_session<T>(
    linter: &Linter,
    filename: &str,
    f: impl FnOnce(&mut CorsaTypeAwareSession) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = linter
        .native_corsa
        .lock()
        .map_err(|_| "Failed to lock corsa type session".to_compact_string())?;

    let needs_new_session = guard
        .as_ref()
        .is_none_or(|session| !session.matches_source_file(filename));

    if needs_new_session {
        if let Some(session) = guard.as_mut() {
            session.close();
        }
        *guard = Some(CorsaTypeAwareSession::new(filename)?);
    }

    let session = guard
        .as_mut()
        .ok_or_else(|| "Failed to initialize corsa type session".to_compact_string())?;

    let result = f(session);
    if result.is_err() {
        session.close();
        *guard = None;
    }
    result
}

fn should_warn_for_prop_access(probe: Option<&TypeProbe>) -> bool {
    let Some(probe) = probe else {
        return true;
    };
    if probe.type_texts.is_empty() {
        return true;
    }
    is_any_like_type_texts(&probe.type_texts) || is_unknown_like_type_texts(&probe.type_texts)
}

fn should_warn_for_emit_validator(probe: Option<&TypeProbe>) -> bool {
    let Some(probe) = probe else {
        return true;
    };
    if probe.call_signatures.is_empty() {
        return true;
    }
    probe.call_signatures.iter().any(|signature| {
        signature.iter().any(|parameter_type| {
            parameter_type.is_empty()
                || is_any_like_type_texts(parameter_type)
                || is_unknown_like_type_texts(parameter_type)
        })
    })
}

fn has_promise_like_return(probe: &TypeProbe) -> bool {
    let no_properties: &[vize_carton::CompactString] = &[];
    probe.return_types.iter().any(|return_type| {
        !return_type.is_empty() && is_promise_like_type_texts(return_type, no_properties)
    })
}

fn is_runtime_array_macro(runtime_args: Option<&str>) -> bool {
    let Some(runtime_args) = runtime_args.map(str::trim_start) else {
        return false;
    };
    if runtime_args.starts_with('[') {
        return true;
    }
    runtime_args
        .find('(')
        .and_then(|open| runtime_args.get(open + 1..))
        .map(str::trim_start)
        .is_some_and(|after_paren| after_paren.starts_with('['))
}

#[derive(Clone, Copy, Debug)]
enum QueryKind {
    PropType,
    EmitValidator,
    Promise,
}

struct MacroQuery {
    kind: QueryKind,
    generated_offset: u32,
    source_start: u32,
    source_end: u32,
}

fn push_promise_marker(
    virtual_ts: &mut VirtualTsOutput,
    script_content: &str,
    source_start: u32,
    source_end: u32,
    queries: &mut Vec<MacroQuery>,
) {
    let Some(insert_offset) = marker_insert_offset(&virtual_ts.content) else {
        return;
    };
    let source_range = source_start as usize..source_end as usize;
    let Some(expression_source) = script_content.get(source_range) else {
        return;
    };

    let mut marker_name = String::with_capacity(28);
    marker_name.push_str("__vize_patina_promise_");
    let query_index = queries.len().to_compact_string();
    marker_name.push_str(query_index.as_str());

    let mut line = String::with_capacity(marker_name.len() + expression_source.len() + 40);
    line.push_str("    function ");
    let name_offset = line.len() as u32;
    line.push_str(&marker_name);
    line.push_str("() { return (");
    line.push_str(expression_source);
    line.push_str("); }\n");

    let generated_offset = insert_offset as u32 + name_offset;
    virtual_ts.content.insert_str(insert_offset, &line);
    queries.push(MacroQuery {
        kind: QueryKind::Promise,
        generated_offset,
        source_start,
        source_end,
    });
}

fn push_prop_type_markers(
    virtual_ts: &mut VirtualTsOutput,
    runtime_args: Option<&str>,
    source_start: u32,
    source_end: u32,
    queries: &mut Vec<MacroQuery>,
) {
    let Some(runtime_args) = runtime_args else {
        return;
    };
    let Some(insert_offset) = marker_insert_offset(&virtual_ts.content) else {
        return;
    };
    let prop_sources = extract_runtime_object_property_values(runtime_args);
    if prop_sources.is_empty() {
        return;
    }

    let mut block = String::with_capacity(runtime_args.len() + 128);
    for (index, prop_source) in prop_sources.iter().enumerate() {
        let mut prop_name = String::with_capacity(24);
        prop_name.push_str("__vize_patina_prop_");
        prop_name.push_str(queries.len().to_compact_string().as_str());
        prop_name.push('_');
        prop_name.push_str(index.to_compact_string().as_str());

        block.push_str("    const ");
        let name_offset = block.len() as u32;
        block.push_str(&prop_name);
        block.push_str(" = (undefined as unknown as __RuntimePropCtor<typeof (");
        block.push_str(prop_source.as_str());
        block.push_str(")>);\n");

        queries.push(MacroQuery {
            kind: QueryKind::PropType,
            generated_offset: insert_offset as u32 + name_offset,
            source_start,
            source_end,
        });
    }

    virtual_ts.content.insert_str(insert_offset, &block);
}

fn push_emit_validator_markers(
    virtual_ts: &mut VirtualTsOutput,
    runtime_args: Option<&str>,
    source_start: u32,
    source_end: u32,
    queries: &mut Vec<MacroQuery>,
) {
    let Some(runtime_args) = runtime_args else {
        return;
    };
    let Some(insert_offset) = marker_insert_offset(&virtual_ts.content) else {
        return;
    };

    let validator_sources = extract_runtime_object_property_values(runtime_args);
    if validator_sources.is_empty() {
        return;
    }

    let mut block = String::with_capacity(runtime_args.len() + 128);
    for (index, validator_source) in validator_sources.iter().enumerate() {
        let mut validator_name = String::with_capacity(24);
        validator_name.push_str("__vize_patina_emit_");
        validator_name.push_str(queries.len().to_compact_string().as_str());
        validator_name.push('_');
        validator_name.push_str(index.to_compact_string().as_str());

        block.push_str("    const ");
        let name_offset = block.len() as u32;
        block.push_str(&validator_name);
        block.push_str(" = (");
        block.push_str(validator_source.as_str());
        block.push_str(");\n");

        queries.push(MacroQuery {
            kind: QueryKind::EmitValidator,
            generated_offset: insert_offset as u32 + name_offset,
            source_start,
            source_end,
        });
    }

    virtual_ts.content.insert_str(insert_offset, &block);
}

fn marker_insert_offset(content: &str) -> Option<usize> {
    content
        .rfind("\n}\n\n// Invoke setup")
        .map(|index| index + 1)
}

fn extract_runtime_object_property_values(source: &str) -> Vec<String> {
    let mut wrapped = String::with_capacity(source.len() + 32);
    wrapped.push_str("const __vize_runtime = (");
    let expression_offset = wrapped.len();
    wrapped.push_str(source);
    wrapped.push_str(");");

    let allocator = OxcAllocator::default();
    let source_type = SourceType::from_path("runtime.ts").unwrap_or_default();
    let parsed = OxcParser::new(&allocator, wrapped.as_str(), source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }

    let Some(Statement::VariableDeclaration(declaration)) = parsed.program.body.first() else {
        return Vec::new();
    };
    let Some(declarator) = declaration.declarations.first() else {
        return Vec::new();
    };
    let Some(init) = declarator.init.as_ref() else {
        return Vec::new();
    };
    let Some(object) = unwrap_object_expression(init) else {
        return Vec::new();
    };

    let mut values = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        if property_name(&property.key).is_none() {
            continue;
        }
        let value_start = property.value.span().start as usize;
        let value_end = property.value.span().end as usize;
        let local_start = value_start.saturating_sub(expression_offset);
        let local_end = value_end.saturating_sub(expression_offset);
        if let Some(value_source) = source.get(local_start..local_end) {
            values.push(value_source.to_compact_string());
        }
    }

    values
}

fn unwrap_object_expression<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match expression {
        Expression::ObjectExpression(object) => Some(object),
        Expression::ParenthesizedExpression(paren) => unwrap_object_expression(&paren.expression),
        Expression::TSAsExpression(ts_as) => unwrap_object_expression(&ts_as.expression),
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            unwrap_object_expression(&ts_satisfies.expression)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            unwrap_object_expression(&ts_non_null.expression)
        }
        _ => None,
    }
}

fn property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(string) => Some(string.value.as_str()),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct FloatingCandidate {
    start: u32,
    end: u32,
}

fn collect_floating_candidates(source: &str) -> Vec<FloatingCandidate> {
    let allocator = OxcAllocator::default();
    let source_type = SourceType::from_path("script.ts").unwrap_or_default();
    let parsed = OxcParser::new(&allocator, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for statement in parsed.program.body.iter() {
        if let Statement::ExpressionStatement(expression_statement) = statement {
            if is_explicitly_handled(&expression_statement.expression) {
                continue;
            }
            let span = expression_statement.expression.span();
            candidates.push(FloatingCandidate {
                start: span.start,
                end: span.end,
            });
        }
    }
    candidates
}

fn is_explicitly_handled(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::AwaitExpression(_) => true,
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::Void,
        Expression::ParenthesizedExpression(paren) => is_explicitly_handled(&paren.expression),
        Expression::TSAsExpression(ts_as) => is_explicitly_handled(&ts_as.expression),
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            is_explicitly_handled(&ts_satisfies.expression)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            is_explicitly_handled(&ts_non_null.expression)
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                is_macro_call_expression(&call.callee) || is_handled_call(call)
            }
            ChainElement::TSNonNullExpression(non_null) => {
                is_explicitly_handled(&non_null.expression)
            }
            _ => false,
        },
        Expression::CallExpression(call) => {
            is_macro_call_expression(&call.callee) || is_handled_call(call)
        }
        _ => false,
    }
}

fn is_handled_call(call: &CallExpression<'_>) -> bool {
    call.callee
        .as_member_expression()
        .and_then(|member| member.static_property_name())
        .is_some_and(|name| matches!(name, "then" | "catch" | "finally"))
}

fn is_macro_call_expression(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => matches!(
            identifier.name.as_str(),
            "defineProps"
                | "defineEmits"
                | "defineExpose"
                | "defineOptions"
                | "defineSlots"
                | "defineModel"
                | "withDefaults"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        has_active_type_aware_rules, lint_sfc_with_corsa, RULE_NO_FLOATING_PROMISES,
        RULE_REQUIRE_TYPED_EMITS, RULE_REQUIRE_TYPED_PROPS,
    };
    use crate::{LintPreset, Linter};

    fn corsa_available() -> bool {
        let mut session = match super::CorsaTypeAwareSession::new("Component.vue") {
            Ok(session) => session,
            Err(_) => return false,
        };
        if session.open_virtual_project("const value = 1;\n").is_err() {
            session.close();
            return false;
        }
        session.close();
        true
    }

    #[test]
    fn opinionated_preset_enables_native_type_aware_rules() {
        let linter = Linter::with_preset(LintPreset::Opinionated);
        assert!(has_active_type_aware_rules(&linter));
    }

    #[test]
    fn require_typed_props_uses_corsa() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
defineProps(['msg', 'count'])
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        assert!(result
            .diagnostics
            .iter()
            .any(|diag| diag.rule_name == RULE_REQUIRE_TYPED_PROPS));
    }

    #[test]
    fn require_typed_emits_uses_corsa() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
defineEmits(['save'])
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        assert!(result
            .diagnostics
            .iter()
            .any(|diag| diag.rule_name == RULE_REQUIRE_TYPED_EMITS));
    }

    #[test]
    fn no_floating_promises_uses_corsa() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
async function loadData(): Promise<number> {
  return 1
}

loadData()
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        assert!(result
            .diagnostics
            .iter()
            .any(|diag| diag.rule_name == RULE_NO_FLOATING_PROMISES));
    }

    #[test]
    fn voided_promises_are_ignored() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
async function loadData(): Promise<number> {
  return 1
}

void loadData()
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        assert!(!result
            .diagnostics
            .iter()
            .any(|diag| diag.rule_name == RULE_NO_FLOATING_PROMISES));
    }

    #[test]
    fn runtime_validators_are_treated_as_typed() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
defineProps({
  msg: { type: String, required: true },
  count: { type: Number, default: 0 },
})

defineEmits({
  save: (value: number) => typeof value === 'number',
})
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        assert!(!result.diagnostics.iter().any(|diag| matches!(
            diag.rule_name,
            RULE_REQUIRE_TYPED_PROPS | RULE_REQUIRE_TYPED_EMITS
        )));
    }

    #[test]
    fn type_aware_diagnostics_snapshot() {
        if !corsa_available() {
            return;
        }

        let linter = Linter::with_preset(LintPreset::Opinionated);
        let source = r#"<script setup lang="ts">
defineProps(['msg'])
defineEmits(['save'])

async function loadData(): Promise<number> {
  return 1
}

loadData()
</script>"#;
        let result = lint_sfc_with_corsa(&linter, source, "Component.vue");
        let diagnostics = result
            .diagnostics
            .iter()
            .map(|diag| {
                (
                    diag.rule_name,
                    diag.message.as_str(),
                    diag.start,
                    diag.end,
                    diag.help.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        insta::assert_debug_snapshot!(diagnostics);
    }
}
