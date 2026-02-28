//! Effect generation and inline operation helpers.

use crate::ir::{IREffect, OperationNode};
use vize_carton::FxHashMap;

use super::{context::GenerateContext, operations::generate_operation, setup::is_svg_tag};

/// Generate effect
pub(crate) fn generate_effect(
    ctx: &mut GenerateContext,
    effect: &IREffect<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("renderEffect");

    // If only one operation, use single-line format
    if effect.operations.len() == 1 {
        let op = &effect.operations[0];
        let op_code = generate_operation_inline(ctx, op);
        ctx.push_line_fmt(format_args!("_renderEffect(() => {})", op_code));
    } else {
        ctx.push_line("_renderEffect(() => {");
        ctx.indent();

        for op in effect.operations.iter() {
            generate_operation(ctx, op, element_template_map);
        }

        ctx.deindent();
        ctx.push_line("})");
    }
}

/// Generate operation inline (returns code string)
pub(crate) fn generate_operation_inline(
    ctx: &mut GenerateContext,
    op: &OperationNode<'_>,
) -> String {
    match op {
        OperationNode::SetProp(set_prop) => {
            let element = vize_carton::new_string!("n{}", set_prop.element);
            let key = &set_prop.prop.key.content;
            let is_svg = is_svg_tag(set_prop.tag.as_str());
            let value = if let Some(first) = set_prop.prop.values.first() {
                if first.is_static {
                    vize_carton::new_string!("\"{}\"", first.content)
                } else {
                    vize_carton::new_string!("_ctx.{}", first.content)
                }
            } else {
                vize_carton::CompactString::from("undefined")
            };

            if key.as_str() == "class" {
                if is_svg {
                    ctx.use_helper("setAttr");
                    vize_carton::new_string!("_setAttr({}, \"class\", {})", element, value).into()
                } else {
                    ctx.use_helper("setClass");
                    vize_carton::new_string!("_setClass({}, {})", element, value).into()
                }
            } else if key.as_str() == "style" {
                if is_svg {
                    ctx.use_helper("setAttr");
                    vize_carton::new_string!("_setAttr({}, \"style\", {})", element, value).into()
                } else {
                    ctx.use_helper("setStyle");
                    vize_carton::new_string!("_setStyle({}, {})", element, value).into()
                }
            } else {
                ctx.use_helper("setProp");
                vize_carton::new_string!("_setProp({}, \"{}\", {})", element, key, value).into()
            }
        }
        OperationNode::SetText(set_text) => {
            ctx.use_helper("setText");
            let text_ref = if let Some(text_var) = ctx.text_nodes.get(&set_text.element) {
                text_var.clone()
            } else {
                vize_carton::new_string!("n{}", set_text.element).into()
            };

            let values: Vec<String> = set_text
                .values
                .iter()
                .map(|v| {
                    ctx.use_helper("toDisplayString");
                    if v.is_static {
                        vize_carton::new_string!("\"{}\"", v.content).into()
                    } else {
                        vize_carton::new_string!("_toDisplayString(_ctx.{})", v.content).into()
                    }
                })
                .collect();

            if values.len() == 1 {
                vize_carton::new_string!("_setText({}, {})", text_ref, values[0]).into()
            } else {
                vize_carton::new_string!("_setText({}, {})", text_ref, values.join(" + ")).into()
            }
        }
        _ => String::from("/* unsupported */"),
    }
}
