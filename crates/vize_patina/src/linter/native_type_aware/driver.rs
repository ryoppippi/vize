use super::{
    has_promise_like_return, push_warning, should_warn_for_emit_validator,
    should_warn_for_prop_access, with_corsa_session, LintResult, Linter, RULE_NO_FLOATING_PROMISES,
    RULE_REQUIRE_TYPED_EMITS, RULE_REQUIRE_TYPED_PROPS,
};
use crate::diagnostic::LintDiagnostic;
use std::path::Path;
use vize_armature::Parser as TemplateParser;
use vize_croquis::{virtual_ts::generate_virtual_ts, Analyzer, AnalyzerOptions};

use super::{
    markers::{push_promise_marker, QueryKind},
    parsing::collect_floating_candidates,
    rule_queries::{collect_emit_queries, collect_prop_queries, push_macro_warning, MacroWarning},
};

pub(super) fn lint_with_descriptor<'a>(
    linter: &Linter,
    source: &str,
    filename: &str,
    descriptor: vize_atelier_sfc::SfcDescriptor<'a>,
) -> LintResult {
    let mut result =
        super::super::script_rules::lint_with_descriptor(linter, filename, &descriptor);

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
    collect_prop_queries(
        linter,
        &analysis,
        &mut result,
        script_block,
        &mut virtual_ts,
        &mut macro_queries,
    );
    collect_emit_queries(
        linter,
        &analysis,
        &mut result,
        script_block,
        &mut virtual_ts,
        &mut macro_queries,
    );

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
                            || corsa::utils::is_promise_like_type_texts(
                                &probe.type_texts,
                                &probe.property_names,
                            )
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

    push_macro_warning(
        &mut result,
        &macro_queries,
        MacroWarning {
            kind: QueryKind::PropType,
            base_offset: script_block.loc.start as u32,
            rule_name: RULE_REQUIRE_TYPED_PROPS,
            message: "Prop should have a type definition",
            help:
                "Use `defineProps<Props>()` or a runtime prop object with concrete constructor types.",
            should_warn: should_warn_for_props,
        },
    );
    push_macro_warning(
        &mut result,
        &macro_queries,
        MacroWarning {
            kind: QueryKind::EmitValidator,
            base_offset: script_block.loc.start as u32,
            rule_name: RULE_REQUIRE_TYPED_EMITS,
            message: "Emit should have a type definition",
            help: "Use `defineEmits<...>()` or a validator object with typed payload parameters.",
            should_warn: should_warn_for_emits,
        },
    );

    result
}
