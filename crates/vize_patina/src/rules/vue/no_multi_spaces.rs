//! vue/no-multi-spaces
//!
//! Disallow multiple consecutive spaces in template.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div  class="foo"></div>
//! <div class="foo"  id="bar"></div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div class="foo"></div>
//! <div class="foo" id="bar"></div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::{Fix, LintDiagnostic, Severity, TextEdit};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-multi-spaces",
    description: "Disallow multiple consecutive spaces",
    category: RuleCategory::StronglyRecommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Disallow multiple spaces
pub struct NoMultiSpaces {
    /// Ignore properties (v-if, v-for expressions)
    pub ignore_properties: bool,
}

impl Default for NoMultiSpaces {
    fn default() -> Self {
        Self {
            ignore_properties: true,
        }
    }
}

impl Rule for NoMultiSpaces {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Check spacing between attributes
        let props: Vec<_> = element.props.iter().collect();

        for i in 0..props.len() {
            if i > 0 {
                let prev_end = prop_gap_start_offset(props[i - 1], ctx.source);
                let curr_start = prop_start_offset(props[i]);
                if curr_start <= prev_end {
                    continue;
                }

                let gap_start = prev_end as usize;
                let gap_end = curr_start as usize;
                let gap = &ctx.source[gap_start..gap_end];

                // Ignore line breaks used for multiline attribute formatting.
                if gap.as_bytes().contains(&b'\n') || gap.as_bytes().contains(&b'\r') {
                    continue;
                }

                if gap.len() > 1 {
                    // More than one space between attributes
                    let fix = Fix::new(
                        "Replace multiple spaces with single space",
                        TextEdit::replace(prev_end, curr_start, " "),
                    );

                    ctx.report(
                        LintDiagnostic::warn(
                            META.name,
                            "Multiple consecutive spaces",
                            prev_end,
                            curr_start,
                        )
                        .with_fix(fix),
                    );
                }
            }
        }
    }
}

fn prop_start_offset(prop: &PropNode<'_>) -> u32 {
    match prop {
        PropNode::Attribute(attr) => attr.name_loc.start.offset,
        PropNode::Directive(dir) => dir.loc.start.offset,
    }
}

fn prop_gap_start_offset(prop: &PropNode<'_>, source: &str) -> u32 {
    let end = prop.loc().end.offset;

    match source.as_bytes().get(end as usize) {
        Some(b'"') | Some(b'\'') => end + 1,
        _ => end,
    }
}

#[cfg(test)]
mod tests {
    use super::{prop_gap_start_offset, NoMultiSpaces};
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;
    use vize_armature::parse;
    use vize_carton::Bump;
    use vize_relief::ast::TemplateChildNode;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoMultiSpaces::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_single_space() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo" id="bar"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_multiple_spaces() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo"  id="bar"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].has_fix());
    }

    #[test]
    fn test_valid_multiline_attributes() {
        let linter = create_linter();
        let result =
            linter.lint_template("<div\n  class=\"foo\"\n  id=\"bar\"\n></div>", "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_indented_multiline_attributes() {
        let linter = create_linter();
        let result = linter.lint_template(
            "<button\n    class=\"btn\"\n    :disabled=\"disabled\"\n  ></button>",
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_prop_gap_start_offset_skips_closing_quote() {
        let allocator = Bump::new();
        let source = r#"<div class="foo" id="bar"></div>"#;
        let (root, errors) = parse(&allocator, source);
        assert!(errors.is_empty());

        let TemplateChildNode::Element(element) = &root.children[0] else {
            panic!("Expected element");
        };

        let gap_start = prop_gap_start_offset(&element.props[0], source);
        assert_eq!(gap_start, 16);
    }
}
