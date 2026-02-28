//! Expression and component prop check generation for virtual TypeScript.
//!
//! Handles generating TypeScript code for template expressions (with optional
//! v-if narrowing) and component prop value type assertions.

use super::types::VizeMapping;
use vize_croquis::analysis::ComponentUsage;

/// Generate a template expression with optional v-if narrowing.
///
/// When the expression has a `vif_guard`, wraps it in an if block to enable TypeScript type narrowing.
/// For example, `{{ todo.description }}` inside `v-if="todo.description"` generates:
/// ```typescript
/// if (todo.description) {
///   const __expr_X = todo.description;
/// }
/// ```
pub(crate) fn generate_expression(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    expr: &vize_croquis::TemplateExpression,
    template_offset: u32,
    indent: &str,
) {
    let src_start = (template_offset + expr.start) as usize;
    let src_end = (template_offset + expr.end) as usize;

    if let Some(ref guard) = expr.vif_guard {
        // Wrap in if block for type narrowing
        ts.push_str(&format!("{}if ({}) {{\n", indent, guard));
        let gen_expr_start = ts.len();
        ts.push_str(&format!(
            "{}  void ({}); // {}\n",
            indent,
            expr.content,
            expr.kind.as_str()
        ));
        let gen_expr_end = ts.len();
        mappings.push(VizeMapping {
            gen_range: gen_expr_start..gen_expr_end,
            src_range: src_start..src_end,
        });
        ts.push_str(&format!(
            "{}  // @vize-map: expr -> {}:{}\n",
            indent, src_start, src_end
        ));
        ts.push_str(&format!("{}}}\n", indent));
    } else {
        let gen_expr_start = ts.len();
        ts.push_str(&format!(
            "{}void ({}); // {}\n",
            indent,
            expr.content,
            expr.kind.as_str()
        ));
        let gen_expr_end = ts.len();
        mappings.push(VizeMapping {
            gen_range: gen_expr_start..gen_expr_end,
            src_range: src_start..src_end,
        });
        ts.push_str(&format!(
            "{}// @vize-map: expr -> {}:{}\n",
            indent, src_start, src_end
        ));
    }
}

/// Generate component prop value checks at the given indentation level.
pub(crate) fn generate_component_prop_checks(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    usage: &ComponentUsage,
    idx: usize,
    template_offset: u32,
    indent: &str,
) {
    let component_name = &usage.name;
    for prop in &usage.props {
        if prop.name.as_str() == "key" || prop.name.as_str() == "ref" {
            continue;
        }
        if let Some(ref value) = prop.value {
            if prop.is_dynamic {
                let prop_src_start = (template_offset + prop.start) as usize;
                let prop_src_end = (template_offset + prop.end) as usize;
                ts.push_str(&format!(
                    "{}// @vize-map: prop -> {}:{}\n",
                    indent, prop_src_start, prop_src_end
                ));

                let safe_prop_name = prop.name.replace('-', "_");

                let gen_prop_start = ts.len();
                ts.push_str(&format!(
                    "{}({}) as __{}_{}_prop_{};\n",
                    indent, value, component_name, idx, safe_prop_name
                ));
                let gen_prop_end = ts.len();
                mappings.push(VizeMapping {
                    gen_range: gen_prop_start..gen_prop_end,
                    src_range: prop_src_start..prop_src_end,
                });
            }
        }
    }
}
