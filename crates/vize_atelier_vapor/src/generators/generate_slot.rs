//! Slot code generation for Vapor mode.

use super::block::GenerateContext;
use crate::ir::SlotOutletIRNode;

/// Generate SlotOutlet code
pub fn generate_slot_outlet(ctx: &mut GenerateContext, slot: &SlotOutletIRNode<'_>) {
    let temp = ctx.next_temp();
    let slot_name = if slot.name.is_static {
        vize_carton::new_string!("\"{}\"", slot.name.content)
    } else {
        vize_carton::CompactString::from(slot.name.content.as_str())
    };

    // Generate props for slot
    let props = if slot.props.is_empty() {
        None
    } else {
        let prop_strs: Vec<String> = slot
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
        Some(vize_carton::new_string!("{{ {} }}", prop_strs.join(", ")))
    };

    // Generate fallback if present
    let has_fallback = slot.fallback.is_some();

    if let Some(props_str) = props {
        if has_fallback {
            ctx.push_line_fmt(format_args!(
                "const {} = _renderSlot($slots, {}, {}, () => {{",
                temp, slot_name, props_str
            ));
            ctx.indent();
            // Fallback content would be generated here
            ctx.push_line("/* fallback content */");
            ctx.deindent();
            ctx.push_line("})");
        } else {
            ctx.push_line_fmt(format_args!(
                "const {} = _renderSlot($slots, {}, {})",
                temp, slot_name, props_str
            ));
        }
    } else if has_fallback {
        ctx.push_line_fmt(format_args!(
            "const {} = _renderSlot($slots, {}, {{}}, () => {{",
            temp, slot_name
        ));
        ctx.indent();
        ctx.push_line("/* fallback content */");
        ctx.deindent();
        ctx.push_line("})");
    } else {
        ctx.push_line_fmt(format_args!(
            "const {} = _renderSlot($slots, {})",
            temp, slot_name
        ));
    }
}

/// Generate slot function for component
pub fn generate_slot_function(name: &str, params: Option<&str>, body: &str) -> String {
    if let Some(p) = params {
        vize_carton::new_string!("{}: ({}) => {}", name, p, body).into()
    } else {
        vize_carton::new_string!("{}: () => {}", name, body).into()
    }
}

/// Generate scoped slots object
pub fn generate_scoped_slots(slots: &[(String, Option<String>, String)]) -> String {
    let slot_strs: Vec<String> = slots
        .iter()
        .map(|(name, params, body)| generate_slot_function(name, params.as_deref(), body))
        .collect();

    vize_carton::new_string!("{{ {} }}", slot_strs.join(", ")).into()
}

/// Generate slot props normalization
pub fn generate_normalize_slots(slots_expr: &str) -> String {
    vize_carton::new_string!("_normalizeSlots({})", slots_expr).into()
}

/// Generate dynamic slot name
pub fn generate_dynamic_slot_name(expr: &str) -> String {
    vize_carton::new_string!("[{}]", expr).into()
}

/// Check if slot is dynamic
pub fn is_dynamic_slot_name(name: &str) -> bool {
    name.starts_with('[') && name.ends_with(']')
}

#[cfg(test)]
mod tests {
    use super::{generate_dynamic_slot_name, generate_slot_function, is_dynamic_slot_name};

    #[test]
    fn test_generate_slot_function_no_params() {
        let result = generate_slot_function("default", None, "_n1");
        assert_eq!(result, "default: () => _n1");
    }

    #[test]
    fn test_generate_slot_function_with_params() {
        let result = generate_slot_function("item", Some("{ data }"), "_n1");
        assert_eq!(result, "item: ({ data }) => _n1");
    }

    #[test]
    fn test_is_dynamic_slot_name() {
        assert!(is_dynamic_slot_name("[slotName]"));
        assert!(!is_dynamic_slot_name("default"));
    }

    #[test]
    fn test_generate_dynamic_slot_name() {
        let result = generate_dynamic_slot_name("dynamicName");
        assert_eq!(result, "[dynamicName]");
    }
}
