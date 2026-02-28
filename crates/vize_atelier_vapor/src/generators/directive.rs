//! Directive code generation for Vapor mode.

use super::block::GenerateContext;
use crate::ir::DirectiveIRNode;
use vize_atelier_core::ExpressionNode;

/// Generate Directive code
pub fn generate_directive(ctx: &mut GenerateContext, directive: &DirectiveIRNode<'_>) {
    let element = vize_carton::new_string!("_n{}", directive.element);
    let name = &directive.name;

    let arg = if let Some(ref arg) = directive.dir.arg {
        match arg {
            ExpressionNode::Simple(exp) => {
                if exp.is_static {
                    vize_carton::new_string!("\"{}\"", exp.content)
                } else {
                    vize_carton::CompactString::from(exp.content.as_str())
                }
            }
            ExpressionNode::Compound(c) => vize_carton::CompactString::from(c.loc.source.as_str()),
        }
    } else {
        vize_carton::CompactString::from("undefined")
    };

    let value = if let Some(ref exp) = directive.dir.exp {
        match exp {
            ExpressionNode::Simple(e) => {
                if e.is_static {
                    vize_carton::new_string!("\"{}\"", e.content)
                } else {
                    vize_carton::CompactString::from(e.content.as_str())
                }
            }
            ExpressionNode::Compound(c) => vize_carton::CompactString::from(c.loc.source.as_str()),
        }
    } else {
        vize_carton::CompactString::from("undefined")
    };

    // Generate modifiers object
    let modifiers = if directive.dir.modifiers.is_empty() {
        vize_carton::CompactString::from("{}")
    } else {
        let mod_strs: Vec<vize_carton::CompactString> = directive
            .dir
            .modifiers
            .iter()
            .map(|m| vize_carton::new_string!("{}: true", m.content))
            .collect();
        vize_carton::new_string!("{{ {} }}", mod_strs.join(", "))
    };

    if directive.builtin {
        // Built-in directive
        match name.as_str() {
            "show" => {
                ctx.push_line_fmt(format_args!(
                    "_withDirectives({}, [[_vShow, {}]])",
                    element, value
                ));
            }
            "model" => {
                ctx.push_line_fmt(format_args!(
                    "_withDirectives({}, [[_vModel, {}, {}, {}]])",
                    element, value, arg, modifiers
                ));
            }
            _ => {
                ctx.push_line_fmt(format_args!(
                    "_withDirectives({}, [[_{}, {}, {}, {}]])",
                    element, name, value, arg, modifiers
                ));
            }
        }
    } else {
        // Custom directive
        ctx.push_line_fmt(format_args!(
            "_withDirectives({}, [[_directive_{}, {}, {}, {}]])",
            element, name, value, arg, modifiers
        ));
    }
}

/// Generate directive resolution
pub fn generate_resolve_directive(name: &str) -> String {
    vize_carton::new_string!("_resolveDirective(\"{}\")", name).into()
}

/// Generate v-show directive
pub fn generate_v_show(element_var: &str, value: &str) -> String {
    vize_carton::new_string!("{}.style.display = {} ? '' : 'none'", element_var, value).into()
}

/// Generate v-cloak removal
pub fn generate_v_cloak_removal(element_var: &str) -> String {
    vize_carton::new_string!("{}.removeAttribute('v-cloak')", element_var).into()
}

/// Generate v-pre handling (skip compilation marker)
pub fn is_v_pre_element(_element: &str) -> bool {
    // Elements with v-pre are handled specially during parsing
    false
}

/// Generate withDirectives call
pub fn generate_with_directives(element_var: &str, directives: &[String]) -> String {
    vize_carton::new_string!(
        "_withDirectives({}, [{}])",
        element_var,
        directives.join(", ")
    )
    .into()
}

/// Generate single directive array
pub fn generate_directive_array(
    directive: &str,
    value: &str,
    arg: Option<&str>,
    modifiers: Option<&str>,
) -> String {
    let mut parts = vec![directive.to_string(), value.to_string()];

    if let Some(a) = arg {
        parts.push(a.to_string());
        if let Some(m) = modifiers {
            parts.push(m.to_string());
        }
    } else if let Some(m) = modifiers {
        parts.push(String::from("undefined"));
        parts.push(m.to_string());
    }

    vize_carton::new_string!("[{}]", parts.join(", ")).into()
}

#[cfg(test)]
mod tests {
    use super::{generate_directive_array, generate_resolve_directive, generate_v_show};

    #[test]
    fn test_generate_resolve_directive() {
        let result = generate_resolve_directive("focus");
        assert_eq!(result, "_resolveDirective(\"focus\")");
    }

    #[test]
    fn test_generate_v_show() {
        let result = generate_v_show("_n1", "isVisible");
        assert_eq!(result, "_n1.style.display = isVisible ? '' : 'none'");
    }

    #[test]
    fn test_generate_directive_array_simple() {
        let result = generate_directive_array("_vShow", "isVisible", None, None);
        assert_eq!(result, "[_vShow, isVisible]");
    }

    #[test]
    fn test_generate_directive_array_with_all() {
        let result =
            generate_directive_array("_vModel", "text", Some("\"value\""), Some("{ lazy: true }"));
        assert_eq!(result, "[_vModel, text, \"value\", { lazy: true }]");
    }
}
