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
use vize_carton::FxHashMap;

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
    let element = format!("n{}", set_prop.element);
    let key = &set_prop.prop.key.content;
    let is_svg = is_svg_tag(set_prop.tag.as_str());

    let value = if let Some(first) = set_prop.prop.values.first() {
        if first.is_static {
            format!("\"{}\"", first.content)
        } else {
            format!("_ctx.{}", first.content)
        }
    } else {
        String::from("undefined")
    };

    if key.as_str() == "class" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line(&format!("_setAttr({}, \"class\", {})", element, value));
        } else {
            ctx.use_helper("setClass");
            ctx.push_line(&format!("_setClass({}, {})", element, value));
        }
    } else if key.as_str() == "style" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line(&format!("_setAttr({}, \"style\", {})", element, value));
        } else {
            ctx.use_helper("setStyle");
            ctx.push_line(&format!("_setStyle({}, {})", element, value));
        }
    } else {
        ctx.use_helper("setProp");
        ctx.push_line(&format!("_setProp({}, \"{}\", {})", element, key, value));
    }
}

/// Generate SetDynamicProps
fn generate_set_dynamic_props(ctx: &mut GenerateContext, set_props: &SetDynamicPropsIRNode<'_>) {
    let element = format!("n{}", set_props.element);

    for prop in set_props.props.iter() {
        let expr = if prop.is_static {
            format!("\"{}\"", prop.content)
        } else {
            prop.content.to_string()
        };
        ctx.push_line(&format!("Object.assign({}, {})", element, expr));
    }
}

/// Generate SetText
fn generate_set_text(ctx: &mut GenerateContext, set_text: &SetTextIRNode<'_>) {
    ctx.use_helper("setText");

    // Use text node reference if available, otherwise use element directly
    let text_ref = if let Some(text_var) = ctx.text_nodes.get(&set_text.element) {
        text_var.clone()
    } else {
        format!("n{}", set_text.element)
    };

    let values: Vec<String> = set_text
        .values
        .iter()
        .map(|v| {
            ctx.use_helper("toDisplayString");
            if v.is_static {
                format!("\"{}\"", v.content)
            } else {
                format!("_toDisplayString(_ctx.{})", v.content)
            }
        })
        .collect();

    if values.len() == 1 {
        ctx.push_line(&format!("_setText({}, {})", text_ref, values[0]));
    } else {
        ctx.push_line(&format!("_setText({}, {})", text_ref, values.join(" + ")));
    }
}

/// Generate SetEvent
fn generate_set_event(ctx: &mut GenerateContext, set_event: &SetEventIRNode<'_>) {
    ctx.use_helper("createInvoker");

    let element = format!("n{}", set_event.element);
    let event_name = &set_event.key.content;

    let handler = if let Some(ref value) = set_event.value {
        value.content.to_string()
    } else {
        String::from("() => {}")
    };

    // Determine handler format based on content
    let invoker_body = if handler.contains("$event") {
        // Handler uses $event - pass it as parameter
        format!("$event => (_ctx.{})", handler)
    } else if handler.contains("?.") {
        // Optional call expression like foo?.() or foo?.bar() - cache it
        format!("(...args) => (_ctx.{})", handler)
    } else if is_inline_statement(&handler) {
        // Inline statement like count++ or foo = bar
        format!("() => (_ctx.{})", handler)
    } else if handler.contains("(") {
        // Handler is a call expression like handler()
        format!("e => _ctx.{}(e)", handler)
    } else {
        // Handler is a method reference like handler
        format!("e => _ctx.{}(e)", handler)
    };

    ctx.push_line(&format!(
        "{}.$evt{} = _createInvoker({})",
        element, event_name, invoker_body
    ));
}

/// Generate SetHtml
fn generate_set_html(ctx: &mut GenerateContext, set_html: &SetHtmlIRNode<'_>) {
    let element = format!("n{}", set_html.element);

    let value = if set_html.value.is_static {
        format!("\"{}\"", set_html.value.content)
    } else {
        set_html.value.content.to_string()
    };

    ctx.push_line(&format!("{}.innerHTML = {}", element, value));
}

/// Generate SetTemplateRef
fn generate_set_template_ref(ctx: &mut GenerateContext, set_ref: &SetTemplateRefIRNode<'_>) {
    let element = format!("n{}", set_ref.element);

    let value = if set_ref.value.is_static {
        format!("\"{}\"", set_ref.value.content)
    } else {
        set_ref.value.content.to_string()
    };

    ctx.push_line(&format!("_setRef({}, {})", element, value));
}

/// Generate InsertNode
fn generate_insert_node(ctx: &mut GenerateContext, insert: &InsertNodeIRNode) {
    let parent = format!("n{}", insert.parent);
    let elements = insert
        .elements
        .iter()
        .map(|e| format!("n{}", e))
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(anchor) = insert.anchor {
        ctx.push_line(&format!("_insert({}, [{}], n{})", parent, elements, anchor));
    } else {
        ctx.push_line(&format!("_insert({}, [{}])", parent, elements));
    }
}

/// Generate PrependNode
fn generate_prepend_node(ctx: &mut GenerateContext, prepend: &PrependNodeIRNode) {
    let parent = format!("n{}", prepend.parent);
    let elements = prepend
        .elements
        .iter()
        .map(|e| format!("n{}", e))
        .collect::<Vec<_>>()
        .join(", ");

    ctx.push_line(&format!("_prepend({}, [{}])", parent, elements));
}

/// Generate Directive
fn generate_directive(ctx: &mut GenerateContext, directive: &DirectiveIRNode<'_>) {
    let element = format!("n{}", directive.element);
    let name = &directive.name;

    let arg = if let Some(ref arg) = directive.dir.arg {
        match arg {
            ExpressionNode::Simple(exp) => {
                if exp.is_static {
                    format!("\"{}\"", exp.content)
                } else {
                    exp.content.to_string()
                }
            }
            _ => String::from("undefined"),
        }
    } else {
        String::from("undefined")
    };

    let value = if let Some(ref exp) = directive.dir.exp {
        match exp {
            ExpressionNode::Simple(e) => {
                if e.is_static {
                    format!("\"{}\"", e.content)
                } else {
                    e.content.to_string()
                }
            }
            _ => String::from("undefined"),
        }
    } else {
        String::from("undefined")
    };

    ctx.push_line(&format!(
        "_withDirectives({}, [[_{}, {}, {}]])",
        element, name, value, arg
    ));
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
        ["(_ctx.", if_node.condition.content.as_str(), ")"].concat()
    };

    ctx.push_line(
        &[
            "const n",
            &if_node.id.to_string(),
            " = _createIf(() => ",
            &condition,
            ", () => {",
        ]
        .concat(),
    );

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
        ["(_ctx.", if_node.condition.content.as_str(), ")"].concat()
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

    let source = if for_node.source.is_static {
        ["\"", for_node.source.content.as_str(), "\""].concat()
    } else {
        ["(_ctx.", for_node.source.content.as_str(), " || [])"].concat()
    };

    let value_name = for_node
        .value
        .as_ref()
        .map(|v| v.content.as_str())
        .unwrap_or("_item");
    let key_name = for_node.key.as_ref().map(|k| k.content.as_str());
    let index_name = for_node.index.as_ref().map(|i| i.content.as_str());

    let params = match (key_name, index_name) {
        (Some(k), Some(i)) => [value_name, ", ", k, ", ", i].concat(),
        (Some(k), None) => [value_name, ", ", k].concat(),
        _ => value_name.to_string(),
    };

    ctx.push_line(&["_createFor(() => ", &source, ", (", &params, ") => {"].concat());
    ctx.indent();
    generate_block(ctx, &for_node.render, element_template_map);
    ctx.deindent();
    ctx.push_line("})");
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
        "null".to_string()
    } else {
        let prop_strs: Vec<String> = component
            .props
            .iter()
            .map(|p| {
                let key = &p.key.content;
                let is_event = key.as_str().starts_with("on") && key.len() > 2;

                let value = if let Some(first) = p.values.first() {
                    if first.is_static {
                        ["() => (\"", first.content.as_str(), "\")"].concat()
                    } else if is_event {
                        // Event handlers: () => _ctx.handler
                        ["() => _ctx.", first.content.as_str()].concat()
                    } else {
                        // Regular props: () => (_ctx.value)
                        ["() => (_ctx.", first.content.as_str(), ")"].concat()
                    }
                } else {
                    "undefined".to_string()
                };
                [key.as_str(), ": ", &value].concat()
            })
            .collect();
        ["{ ", &prop_strs.join(", "), " }"].concat()
    };

    // Generate component creation
    ctx.push_line(
        &[
            "const n",
            &component.id.to_string(),
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
        format!("\"{}\"", slot.name.content)
    } else {
        slot.name.content.to_string()
    };

    ctx.push_line(&format!(
        "const {} = _renderSlot($slots, {})",
        name, slot_name
    ));
}

/// Generate GetTextChild
fn generate_get_text_child(ctx: &mut GenerateContext, get_text: &GetTextChildIRNode) {
    let parent = format!("n{}", get_text.parent);
    let child = ctx.next_temp();

    ctx.push_line(&format!("const {} = {}.firstChild", child, parent));
}

/// Check if handler is an inline statement (not a function reference)
fn is_inline_statement(handler: &str) -> bool {
    // Assignment or increment/decrement operators
    handler.contains("++")
        || handler.contains("--")
        || handler.contains("+=")
        || handler.contains("-=")
        || handler.contains("=")
}
