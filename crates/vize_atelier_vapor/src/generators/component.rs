//! Component code generation for Vapor mode.

use super::block::GenerateContext;
use crate::ir::CreateComponentIRNode;

/// Generate CreateComponent code
pub fn generate_create_component(ctx: &mut GenerateContext, component: &CreateComponentIRNode<'_>) {
    let temp = ctx.next_temp();
    let tag = &component.tag;

    // Generate props object
    let props: String = if component.props.is_empty() {
        String::from("{}")
    } else {
        let prop_strs: Vec<String> = component
            .props
            .iter()
            .map(|p| {
                let key = &p.key.content;
                let value: String = if let Some(first) = p.values.first() {
                    if first.is_static {
                        vize_carton::new_string!("\"{}\"", first.content).into()
                    } else {
                        first.content.to_string()
                    }
                } else {
                    String::from("undefined")
                };
                vize_carton::new_string!("{}: {}", key, value).into()
            })
            .collect();
        vize_carton::new_string!("{{ {} }}", prop_strs.join(", ")).into()
    };

    // Generate slots if present
    let slots_code = if component.slots.is_empty() {
        None
    } else {
        Some(generate_slots_object(component))
    };

    if let Some(slots) = slots_code {
        ctx.push_line_fmt(format_args!(
            "const {} = _createComponent({}, {}, {})",
            temp, tag, props, slots
        ));
    } else {
        ctx.push_line_fmt(format_args!(
            "const {} = _createComponent({}, {})",
            temp, tag, props
        ));
    }
}

/// Generate slots object for component
fn generate_slots_object(component: &CreateComponentIRNode<'_>) -> String {
    let slot_strs: Vec<String> = component
        .slots
        .iter()
        .map(|slot| {
            let name: String = if slot.name.is_static {
                slot.name.content.to_string()
            } else {
                vize_carton::new_string!("[{}]", slot.name.content).into()
            };

            let params = slot
                .fn_exp
                .as_ref()
                .map(|p| p.content.to_string())
                .unwrap_or_default();

            vize_carton::new_string!("{}: ({}) => {{ /* slot content */ }}", name, params).into()
        })
        .collect();

    vize_carton::new_string!("{{ {} }}", slot_strs.join(", ")).into()
}

/// Generate component resolution
pub fn generate_resolve_component(name: &str) -> String {
    vize_carton::new_string!("_resolveComponent(\"{}\")", name).into()
}

/// Generate dynamic component
pub fn generate_dynamic_component(
    component_expr: &str,
    props: &str,
    slots: Option<&str>,
) -> String {
    if let Some(slots_code) = slots {
        vize_carton::new_string!(
            "_createComponent({}, {}, {})",
            component_expr,
            props,
            slots_code
        )
        .into()
    } else {
        vize_carton::new_string!("_createComponent({}, {})", component_expr, props).into()
    }
}

/// Generate async component wrapper
pub fn generate_async_component(component_expr: &str) -> String {
    vize_carton::new_string!("_defineAsyncComponent(() => {})", component_expr).into()
}

/// Generate suspense boundary
pub fn generate_suspense(fallback: Option<&str>) -> (String, String) {
    if let Some(fb) = fallback {
        (
            vize_carton::new_string!("_createSuspense({{ fallback: () => {} }})", fb).into(),
            String::from("})"),
        )
    } else {
        (String::from("_createSuspense({"), String::from("})"))
    }
}

/// Generate keep-alive wrapper
pub fn generate_keep_alive(
    include: Option<&str>,
    exclude: Option<&str>,
    max: Option<usize>,
) -> String {
    let mut options: Vec<vize_carton::CompactString> = Vec::new();

    if let Some(inc) = include {
        options.push(vize_carton::new_string!("include: {}", inc));
    }
    if let Some(exc) = exclude {
        options.push(vize_carton::new_string!("exclude: {}", exc));
    }
    if let Some(m) = max {
        options.push(vize_carton::new_string!("max: {}", m));
    }

    if options.is_empty() {
        String::from("_createKeepAlive({})")
    } else {
        vize_carton::new_string!("_createKeepAlive({{ {} }})", options.join(", ")).into()
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_keep_alive, generate_resolve_component};

    #[test]
    fn test_generate_resolve_component() {
        let result = generate_resolve_component("MyComponent");
        assert_eq!(result, "_resolveComponent(\"MyComponent\")");
    }

    #[test]
    fn test_generate_keep_alive_empty() {
        let result = generate_keep_alive(None, None, None);
        assert_eq!(result, "_createKeepAlive({})");
    }

    #[test]
    fn test_generate_keep_alive_with_options() {
        let result = generate_keep_alive(Some("\"Component1\""), None, Some(10));
        assert!(result.contains("include: \"Component1\""));
        assert!(result.contains("max: 10"));
    }
}
