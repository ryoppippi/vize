//! Individual operation code generators.
//!
//! Each function emits JavaScript code for a specific IR operation node.

use crate::ir::{
    CreateComponentIRNode, DirectiveIRNode, ForIRNode, GetTextChildIRNode, IfIRNode,
    InsertNodeIRNode, NegativeBranch, OperationNode, PrependNodeIRNode, SetDynamicPropsIRNode,
    SetEventIRNode, SetHtmlIRNode, SetPropIRNode, SetTemplateRefIRNode, SetTextIRNode,
    SlotOutletIRNode,
};
use vize_atelier_core::ExpressionNode;
use vize_carton::{cstr, FxHashMap, String, ToCompactString};

use super::{context::GenerateContext, generate_block, setup::is_svg_tag};

/// Generate operation
pub(crate) fn generate_operation(
    ctx: &mut GenerateContext,
    op: &OperationNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    match op {
        OperationNode::SetProp(set_prop) => {
            generate_set_prop(ctx, set_prop);
        }
        OperationNode::SetDynamicProps(set_props) => {
            generate_set_dynamic_props(ctx, set_props);
        }
        OperationNode::SetText(set_text) => {
            generate_set_text(ctx, set_text);
        }
        OperationNode::SetEvent(set_event) => {
            generate_set_event(ctx, set_event);
        }
        OperationNode::SetHtml(set_html) => {
            generate_set_html(ctx, set_html);
        }
        OperationNode::SetTemplateRef(set_ref) => {
            generate_set_template_ref(ctx, set_ref);
        }
        OperationNode::InsertNode(insert) => {
            generate_insert_node(ctx, insert);
        }
        OperationNode::PrependNode(prepend) => {
            generate_prepend_node(ctx, prepend);
        }
        OperationNode::Directive(directive) => {
            generate_directive(ctx, directive);
        }
        OperationNode::If(if_node) => {
            generate_if(ctx, if_node, element_template_map);
        }
        OperationNode::For(for_node) => {
            generate_for(ctx, for_node, element_template_map);
        }
        OperationNode::CreateComponent(component) => {
            generate_create_component(ctx, component);
        }
        OperationNode::SlotOutlet(slot) => {
            generate_slot_outlet(ctx, slot);
        }
        OperationNode::GetTextChild(get_text) => {
            generate_get_text_child(ctx, get_text);
        }
    }
}

/// Generate SetProp
fn generate_set_prop(ctx: &mut GenerateContext, set_prop: &SetPropIRNode<'_>) {
    let element = cstr!("n{}", set_prop.element);
    let key = &set_prop.prop.key.content;
    let is_svg = is_svg_tag(set_prop.tag.as_str());

    let value = if let Some(first) = set_prop.prop.values.first() {
        if first.is_static {
            cstr!("\"{}\"", first.content)
        } else {
            ctx.resolve_expression(&first.content)
        }
    } else {
        vize_carton::CompactString::from("undefined")
    };

    if key.as_str() == "class" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line_fmt(format_args!("_setAttr({element}, \"class\", {value})"));
        } else {
            ctx.use_helper("setClass");
            ctx.push_line_fmt(format_args!("_setClass({element}, {value})"));
        }
    } else if key.as_str() == "style" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line_fmt(format_args!("_setAttr({element}, \"style\", {value})"));
        } else {
            ctx.use_helper("setStyle");
            ctx.push_line_fmt(format_args!("_setStyle({element}, {value})"));
        }
    } else {
        ctx.use_helper("setProp");
        ctx.push_line_fmt(format_args!("_setProp({element}, \"{key}\", {value})"));
    }
}

/// Generate SetDynamicProps
fn generate_set_dynamic_props(ctx: &mut GenerateContext, set_props: &SetDynamicPropsIRNode<'_>) {
    let element = cstr!("n{}", set_props.element);

    if set_props.is_event {
        // v-on="handlers" → _setDynamicEvents
        ctx.use_helper("setDynamicEvents");
        for prop in set_props.props.iter() {
            let resolved = ctx.resolve_expression(&prop.content);
            ctx.push_line_fmt(format_args!("_setDynamicEvents({}, {})", element, resolved));
        }
    } else {
        for prop in set_props.props.iter() {
            let expr = if prop.is_static {
                cstr!("\"{}\"", prop.content)
            } else {
                vize_carton::CompactString::from(prop.content.as_str())
            };
            ctx.push_line_fmt(format_args!("Object.assign({}, {})", element, expr));
        }
    }
}

/// Generate SetText
fn generate_set_text(ctx: &mut GenerateContext, set_text: &SetTextIRNode<'_>) {
    ctx.use_helper("setText");

    // Use text node reference if available, otherwise use element directly
    let text_ref = if let Some(text_var) = ctx.text_nodes.get(&set_text.element) {
        text_var.clone()
    } else {
        cstr!("n{}", set_text.element)
    };

    let values: Vec<String> = set_text
        .values
        .iter()
        .map(|v| {
            if v.is_static {
                cstr!("\"{}\"", v.content)
            } else {
                ctx.use_helper("toDisplayString");
                let resolved = ctx.resolve_expression(&v.content);
                cstr!("_toDisplayString({})", resolved)
            }
        })
        .collect();

    if values.len() == 1 {
        ctx.push_line_fmt(format_args!("_setText({}, {})", text_ref, values[0]));
    } else {
        ctx.push_line_fmt(format_args!(
            "_setText({}, {})",
            text_ref,
            values.join(" + ")
        ));
    }
}

/// Generate SetEvent
fn generate_set_event(ctx: &mut GenerateContext, set_event: &SetEventIRNode<'_>) {
    ctx.use_helper("createInvoker");

    let element = cstr!("n{}", set_event.element);
    let event_name = &set_event.key.content;

    let handler = if let Some(ref value) = set_event.value {
        value.content.to_compact_string()
    } else {
        String::from("() => {}")
    };

    let resolved_handler = ctx.resolve_expression(&handler);
    // Determine handler format based on content
    let invoker_body: String = if handler.contains("$event") {
        cstr!("$event => ({})", resolved_handler)
    } else if handler.contains("?.") {
        cstr!("(...args) => ({})", resolved_handler)
    } else if is_inline_statement(&handler) || handler.contains('(') {
        cstr!("() => ({})", resolved_handler)
    } else {
        cstr!("e => {}(e)", resolved_handler)
    };

    // Wrap with withModifiers if there are DOM modifiers (stop, prevent, etc.)
    let wrapped_handler = if !set_event.modifiers.non_keys.is_empty() {
        ctx.use_helper("withModifiers");
        let mods = set_event
            .modifiers
            .non_keys
            .iter()
            .map(|m| ["\"", m.as_str(), "\""].concat())
            .collect::<std::vec::Vec<_>>()
            .join(",");
        cstr!("_withModifiers({}, [{}])", invoker_body, mods)
    } else if !set_event.modifiers.keys.is_empty() {
        ctx.use_helper("withKeys");
        let keys = set_event
            .modifiers
            .keys
            .iter()
            .map(|k| ["\"", k.as_str(), "\""].concat())
            .collect::<std::vec::Vec<_>>()
            .join(",");
        cstr!("_withKeys({}, [{}])", invoker_body, keys)
    } else {
        invoker_body
    };

    if set_event.delegate {
        // Use delegation
        ctx.push_line_fmt(format_args!(
            "{}.$evt{} = _createInvoker({})",
            element, event_name, wrapped_handler
        ));
    } else if set_event.effect {
        // Dynamic event - use renderEffect + _on
        ctx.use_helper("on");
        ctx.use_helper("renderEffect");
        let event_expr = ctx.resolve_expression(event_name.as_str());
        ctx.push_line("_renderEffect(() => {");
        ctx.indent();
        ctx.push_line("");
        ctx.push_line_fmt(format_args!(
            "_on({}, {}, _createInvoker({}), {{",
            element, event_expr, wrapped_handler
        ));
        ctx.indent();
        ctx.push_line("effect: true");
        ctx.deindent();
        ctx.push_line("})");
        ctx.deindent();
        ctx.push_line("})");
    } else {
        // Use _on() for non-delegatable events or events with once/capture/passive
        ctx.use_helper("on");

        let has_options = set_event.modifiers.options.once
            || set_event.modifiers.options.capture
            || set_event.modifiers.options.passive;

        if has_options {
            let mut opts = std::vec::Vec::new();
            if set_event.modifiers.options.once {
                opts.push("once: true");
            }
            if set_event.modifiers.options.capture {
                opts.push("capture: true");
            }
            if set_event.modifiers.options.passive {
                opts.push("passive: true");
            }
            ctx.push_line_fmt(format_args!(
                "_on({}, \"{}\", _createInvoker({}), {{",
                element, event_name, wrapped_handler
            ));
            ctx.indent();
            for opt in &opts {
                ctx.push_line(opt);
            }
            ctx.deindent();
            ctx.push_line("})");
        } else {
            ctx.push_line_fmt(format_args!(
                "_on({}, \"{}\", _createInvoker({}))",
                element, event_name, wrapped_handler
            ));
        }
    }
}

/// Generate SetHtml
fn generate_set_html(ctx: &mut GenerateContext, set_html: &SetHtmlIRNode<'_>) {
    let element = cstr!("n{}", set_html.element);

    let value = if set_html.value.is_static {
        cstr!("\"{}\"", set_html.value.content)
    } else {
        vize_carton::CompactString::from(set_html.value.content.as_str())
    };

    ctx.push_line_fmt(format_args!("{}.innerHTML = {}", element, value));
}

/// Generate SetTemplateRef
fn generate_set_template_ref(ctx: &mut GenerateContext, set_ref: &SetTemplateRefIRNode<'_>) {
    let element = cstr!("n{}", set_ref.element);

    let value = if set_ref.value.is_static {
        cstr!("\"{}\"", set_ref.value.content)
    } else {
        vize_carton::CompactString::from(set_ref.value.content.as_str())
    };

    ctx.push_line_fmt(format_args!("_setRef({}, {})", element, value));
}

/// Generate InsertNode
fn generate_insert_node(ctx: &mut GenerateContext, insert: &InsertNodeIRNode) {
    let parent = cstr!("n{}", insert.parent);
    let elements = insert
        .elements
        .iter()
        .map(|e| cstr!("n{e}"))
        .collect::<std::vec::Vec<_>>()
        .join(", ");

    if let Some(anchor) = insert.anchor {
        ctx.push_line_fmt(format_args!(
            "_insert({}, [{}], n{})",
            parent, elements, anchor
        ));
    } else {
        ctx.push_line_fmt(format_args!("_insert({}, [{}])", parent, elements));
    }
}

/// Generate PrependNode
fn generate_prepend_node(ctx: &mut GenerateContext, prepend: &PrependNodeIRNode) {
    let parent = cstr!("n{}", prepend.parent);
    let elements = prepend
        .elements
        .iter()
        .map(|e| cstr!("n{e}"))
        .collect::<std::vec::Vec<_>>()
        .join(", ");

    ctx.push_line_fmt(format_args!("_prepend({}, [{}])", parent, elements));
}

/// Generate Directive
fn generate_directive(ctx: &mut GenerateContext, directive: &DirectiveIRNode<'_>) {
    let element = cstr!("n{}", directive.element);

    // Handle v-show
    if directive.name.as_str() == "vShow" {
        ctx.use_helper("applyVShow");
        let value = if let Some(ref exp) = directive.dir.exp {
            match exp {
                ExpressionNode::Simple(e) => {
                    if e.is_static {
                        cstr!("\"{}\"", e.content)
                    } else {
                        ctx.resolve_expression(&e.content)
                    }
                }
                _ => vize_carton::CompactString::from("undefined"),
            }
        } else {
            vize_carton::CompactString::from("undefined")
        };
        ctx.push_line_fmt(format_args!("_applyVShow({}, () => ({}))", element, value));
        return;
    }

    // Handle v-model on elements
    if directive.name.as_str() == "model" {
        generate_v_model(ctx, directive);
        return;
    }

    let name = &directive.name;

    let arg = if let Some(ref arg) = directive.dir.arg {
        match arg {
            ExpressionNode::Simple(exp) => {
                if exp.is_static {
                    cstr!("\"{}\"", exp.content)
                } else {
                    vize_carton::CompactString::from(exp.content.as_str())
                }
            }
            _ => vize_carton::CompactString::from("undefined"),
        }
    } else {
        vize_carton::CompactString::from("undefined")
    };

    let value = if let Some(ref exp) = directive.dir.exp {
        match exp {
            ExpressionNode::Simple(e) => {
                if e.is_static {
                    cstr!("\"{}\"", e.content)
                } else {
                    vize_carton::CompactString::from(e.content.as_str())
                }
            }
            _ => vize_carton::CompactString::from("undefined"),
        }
    } else {
        vize_carton::CompactString::from("undefined")
    };

    ctx.push_line_fmt(format_args!(
        "_withDirectives({}, [[_{}, {}, {}]])",
        element, name, value, arg
    ));
}

/// Generate v-model for element
fn generate_v_model(ctx: &mut GenerateContext, directive: &DirectiveIRNode<'_>) {
    let element = cstr!("n{}", directive.element);

    let binding = if let Some(ref exp) = directive.dir.exp {
        match exp {
            ExpressionNode::Simple(e) => e.content.clone(),
            _ => vize_carton::String::from(""),
        }
    } else {
        vize_carton::String::from("")
    };

    let helper = if directive.tag.as_str() == "select" {
        "applySelectModel"
    } else if directive.tag.as_str() == "textarea" {
        "applyTextModel"
    } else if directive.tag.as_str() == "input" {
        match directive.input_type.as_str() {
            "checkbox" => "applyCheckboxModel",
            "radio" => "applyRadioModel",
            _ => "applyTextModel",
        }
    } else {
        "applyTextModel"
    };

    ctx.use_helper(helper);

    // Build modifiers options
    let modifiers = &directive.dir.modifiers;
    let mut mod_parts: std::vec::Vec<String> = std::vec::Vec::new();
    for m in modifiers.iter() {
        match m.content.as_str() {
            "lazy" => mod_parts.push("lazy: true".into()),
            "number" => mod_parts.push("number: true".into()),
            "trim" => mod_parts.push("trim: true".into()),
            _ => {}
        }
    }

    if mod_parts.is_empty() {
        ctx.push_line_fmt(format_args!(
            "_{}({}, () => (_ctx.{}), _value => (_ctx.{} = _value))",
            helper, element, binding, binding
        ));
    } else {
        ctx.push_line_fmt(format_args!(
            "_{}({}, () => (_ctx.{}), _value => (_ctx.{} = _value), {{ {} }})",
            helper,
            element,
            binding,
            binding,
            mod_parts.join(",")
        ));
    }
}

/// Generate If
fn generate_if(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    generate_if_inner(ctx, if_node, element_template_map);
}

/// Generate If (inner - for top-level if nodes)
fn generate_if_inner(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createIf");

    let condition = if if_node.condition.is_static {
        ["\"", if_node.condition.content.as_str(), "\""].concat()
    } else {
        let resolved = ctx.resolve_expression(&if_node.condition.content);
        ["(", &resolved, ")"].concat()
    };

    ctx.push_line(
        &[
            "const n",
            &if_node.id.to_compact_string(),
            " = _createIf(() => ",
            &condition,
            ", () => {",
        ]
        .concat(),
    );

    let was_fragment = ctx.is_fragment;
    ctx.is_fragment = true;
    ctx.indent();
    generate_block(ctx, &if_node.positive, element_template_map);
    ctx.deindent();

    if let Some(ref negative) = if_node.negative {
        match negative {
            NegativeBranch::Block(block) => {
                ctx.push_line("}, () => {");
                ctx.indent();
                generate_block(ctx, block, element_template_map);
                ctx.deindent();
                ctx.push_line("})");
            }
            NegativeBranch::If(nested_if) => {
                // v-else-if: inline format without block wrapper
                ctx.push_indent();
                ctx.push("}, () => ");
                generate_nested_if(ctx, nested_if, element_template_map);
                ctx.push(")\n");
            }
        }
    } else {
        ctx.push_line("})");
    }
    ctx.is_fragment = was_fragment;
}

/// Generate nested if (for v-else-if chains - starts inline without leading indent)
fn generate_nested_if(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createIf");

    let condition = if if_node.condition.is_static {
        ["\"", if_node.condition.content.as_str(), "\""].concat()
    } else {
        let resolved = ctx.resolve_expression(&if_node.condition.content);
        ["(", &resolved, ")"].concat()
    };

    // Start inline - no leading indent or newline
    ctx.push(&["_createIf(() => ", &condition, ", () => {\n"].concat());

    ctx.indent();
    generate_block(ctx, &if_node.positive, element_template_map);
    ctx.deindent();

    if let Some(ref negative) = if_node.negative {
        match negative {
            NegativeBranch::Block(block) => {
                ctx.push_line("}, () => {");
                ctx.indent();
                generate_block(ctx, block, element_template_map);
                ctx.deindent();
                ctx.push_indent();
                ctx.push("})");
            }
            NegativeBranch::If(nested_if) => {
                ctx.push_indent();
                ctx.push("}, () => ");
                generate_nested_if(ctx, nested_if, element_template_map);
                ctx.push(")");
            }
        }
    } else {
        ctx.push_indent();
        ctx.push("})");
    }
}

/// Generate For
fn generate_for(
    ctx: &mut GenerateContext,
    for_node: &ForIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createFor");

    let depth = ctx.for_scopes.len();
    let source = if for_node.source.is_static {
        ["(", for_node.source.content.as_str(), ")"].concat()
    } else {
        let resolved = ctx.resolve_expression(&for_node.source.content);
        ["(", &resolved, ")"].concat()
    };

    let value_alias = for_node.value.as_ref().map(|v| v.content.clone());
    let key_alias = for_node.key.as_ref().map(|k| k.content.clone());

    // Build parameter list using _for_item0, _for_key0 naming
    let for_item_var = cstr!("_for_item{}", depth);
    let for_key_var = cstr!("_for_key{}", depth);

    let params: String = if key_alias.is_some() {
        [for_item_var.as_str(), ", ", for_key_var.as_str()]
            .concat()
            .into()
    } else {
        for_item_var.clone()
    };

    // Push for scope before generating body
    let scope = super::context::ForScope {
        value_alias: value_alias.clone(),
        key_alias: key_alias.clone(),
        index_alias: for_node.index.as_ref().map(|i| i.content.clone()),
        depth,
    };
    ctx.for_scopes.push(scope);

    let was_fragment = ctx.is_fragment;
    ctx.is_fragment = true;

    let for_id_str = for_node.id.to_compact_string();
    ctx.push_line(
        &[
            "const n",
            &for_id_str,
            " = _createFor(() => ",
            &source,
            ", (",
            &params,
            ") => {",
        ]
        .concat(),
    );
    ctx.indent();
    generate_block(ctx, &for_node.render, element_template_map);
    ctx.deindent();

    // Generate key function if key_prop is provided
    let key_func = generate_for_key_function(for_node);

    // Check if this is a range-based for (source is a number literal)
    let is_range = for_node.source.content.as_str().parse::<f64>().is_ok();

    // Determine memo flag: 4 = range, 1 = only child of parent (nested v-for)
    let memo_flag = if is_range {
        Some("4")
    } else if for_node.only_child && was_fragment {
        // only_child flag is for nested v-for inside another element
        Some("1")
    } else {
        None
    };

    if let Some(key_fn) = key_func {
        if let Some(flag) = memo_flag {
            ctx.push_line(&["}, ", &key_fn, ", ", flag, ")"].concat());
        } else {
            ctx.push_line(&["}, ", &key_fn, ")"].concat());
        }
    } else {
        ctx.push_line("})");
    }

    ctx.is_fragment = was_fragment;
    ctx.for_scopes.pop();
}

/// Generate key function for v-for
fn generate_for_key_function(for_node: &ForIRNode<'_>) -> Option<String> {
    if let Some(ref key_prop) = for_node.key_prop {
        let key_expr = &key_prop.content;
        // Build params: (value_alias) or (value_alias, key_alias)
        let value_name = for_node
            .value
            .as_ref()
            .map(|v| v.content.as_str())
            .unwrap_or("_item");
        let key_name = for_node.key.as_ref().map(|k| k.content.as_str());

        let params = if let Some(k) = key_name {
            [value_name, ", ", k].concat()
        } else {
            value_name.to_compact_string().into()
        };

        Some(cstr!("({params}) => ({key_expr})"))
    } else {
        None
    }
}

/// Generate CreateComponent
fn generate_create_component(ctx: &mut GenerateContext, component: &CreateComponentIRNode<'_>) {
    ctx.use_helper("resolveComponent");
    ctx.use_helper("createComponentWithFallback");

    let tag = &component.tag;
    let component_var = ["_component_", tag.as_str()].concat();

    // Resolve component
    ctx.push_line(
        &[
            "const ",
            &component_var,
            " = _resolveComponent(\"",
            tag.as_str(),
            "\")",
        ]
        .concat(),
    );

    // Props object
    let props = if component.props.is_empty() {
        "null".to_compact_string()
    } else {
        let prop_strs: Vec<String> = component
            .props
            .iter()
            .map(|p| {
                let key = &p.key.content;
                let is_event = key.as_str().starts_with("on") && key.len() > 2;

                let value: String = if let Some(first) = p.values.first() {
                    if first.content.starts_with("__RAW__") {
                        String::from(&first.content.as_str()[7..])
                    } else if first.is_static {
                        ["() => (\"", first.content.as_str(), "\")"].concat().into()
                    } else if is_event {
                        ["() => _ctx.", first.content.as_str()].concat().into()
                    } else {
                        ["() => (_ctx.", first.content.as_str(), ")"]
                            .concat()
                            .into()
                    }
                } else {
                    "undefined".to_compact_string()
                };
                if key.contains(':') {
                    ["\"", key.as_str(), "\": ", &value].concat().into()
                } else {
                    [key.as_str(), ": ", &value].concat().into()
                }
            })
            .collect();
        // Use multi-line format for 2+ props
        if prop_strs.len() >= 2 {
            let mut result = String::from("{\n");
            for (i, prop_str) in prop_strs.iter().enumerate() {
                result.push_str("    ");
                result.push_str(prop_str);
                if i < prop_strs.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str("  }");
            result
        } else {
            ["{ ", &prop_strs.join(", "), " }"].concat().into()
        }
    };

    // Generate component creation
    ctx.push_line(
        &[
            "const n",
            &component.id.to_compact_string(),
            " = _createComponentWithFallback(",
            &component_var,
            ", ",
            &props,
            ", null, true)",
        ]
        .concat(),
    );
}

/// Generate SlotOutlet
fn generate_slot_outlet(ctx: &mut GenerateContext, slot: &SlotOutletIRNode<'_>) {
    let name = ctx.next_temp();
    let slot_name = if slot.name.is_static {
        cstr!("\"{}\"", slot.name.content)
    } else {
        vize_carton::CompactString::from(slot.name.content.as_str())
    };

    ctx.push_line_fmt(format_args!(
        "const {} = _renderSlot($slots, {})",
        name, slot_name
    ));
}

/// Generate GetTextChild
fn generate_get_text_child(ctx: &mut GenerateContext, get_text: &GetTextChildIRNode) {
    let parent = cstr!("n{}", get_text.parent);
    let child = ctx.next_temp();

    ctx.push_line_fmt(format_args!("const {} = {}.firstChild", child, parent));
}

/// Check if handler is an inline statement (not a function reference)
fn is_inline_statement(handler: &str) -> bool {
    // Assignment or increment/decrement operators
    handler.contains("++")
        || handler.contains("--")
        || handler.contains("+=")
        || handler.contains("-=")
        || (handler.contains('=') && !handler.contains("==") && !handler.contains("=>"))
}
