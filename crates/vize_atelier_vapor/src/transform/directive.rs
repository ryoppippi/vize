//! Directive transformation.
//!
//! Handles v-bind, v-on, v-if, v-for, v-html, v-text, and custom directives.

use vize_carton::{Box, Vec};

use crate::ir::{
    BlockIRNode, DirectiveIRNode, ForIRNode, IREffect, IRProp, IfIRNode, OperationNode,
    SetEventIRNode, SetHtmlIRNode, SetPropIRNode, SetTextIRNode,
};
use vize_atelier_core::{
    DirectiveNode, ElementNode, ElementType, ExpressionNode, PropNode, SimpleExpressionNode,
};

use super::{context::TransformContext, transform_children};

/// Transform directive
pub(crate) fn transform_directive<'a>(
    ctx: &mut TransformContext<'a>,
    dir: &DirectiveNode<'a>,
    element_id: usize,
    el: &ElementNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    match dir.name.as_str() {
        "bind" => {
            // Skip :key - handled by v-for key function
            if let Some(ref arg) = dir.arg {
                if let ExpressionNode::Simple(key_exp) = arg {
                    if key_exp.content.as_str() == "key" {
                        return;
                    }
                }
            }
            // v-bind - SetProp
            if let Some(ref arg) = dir.arg {
                if let ExpressionNode::Simple(key_exp) = arg {
                    let key_node = SimpleExpressionNode::new(
                        key_exp.content.clone(),
                        key_exp.is_static,
                        key_exp.loc.clone(),
                    );
                    let key = Box::new_in(key_node, ctx.allocator);

                    let values = if let Some(ref exp) = dir.exp {
                        if let ExpressionNode::Simple(val_exp) = exp {
                            let mut v = Vec::new_in(ctx.allocator);
                            let val_node = SimpleExpressionNode::new(
                                val_exp.content.clone(),
                                val_exp.is_static,
                                val_exp.loc.clone(),
                            );
                            v.push(Box::new_in(val_node, ctx.allocator));
                            v
                        } else {
                            Vec::new_in(ctx.allocator)
                        }
                    } else {
                        Vec::new_in(ctx.allocator)
                    };

                    let set_prop = SetPropIRNode {
                        element: element_id,
                        prop: IRProp {
                            key,
                            values,
                            is_component: el.tag_type == ElementType::Component,
                        },
                        tag: el.tag.clone(),
                    };

                    // Reactive prop - add to effects
                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetProp(set_prop));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        "on" => {
            // v-on - SetEvent
            if let Some(ref arg) = dir.arg {
                if let ExpressionNode::Simple(key_exp) = arg {
                    let key_node = SimpleExpressionNode::new(
                        key_exp.content.clone(),
                        key_exp.is_static,
                        key_exp.loc.clone(),
                    );
                    let key = Box::new_in(key_node, ctx.allocator);

                    let value = if let Some(ref exp) = dir.exp {
                        if let ExpressionNode::Simple(val_exp) = exp {
                            let val_node = SimpleExpressionNode::new(
                                val_exp.content.clone(),
                                val_exp.is_static,
                                val_exp.loc.clone(),
                            );
                            Some(Box::new_in(val_node, ctx.allocator))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let set_event = SetEventIRNode {
                        element: element_id,
                        key,
                        value,
                        modifiers: Default::default(),
                        delegate: true,
                        effect: false,
                    };

                    block.operation.push(OperationNode::SetEvent(set_event));
                }
            }
        }
        "if" => {
            // v-if
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(cond_exp) = exp {
                    let cond_node = SimpleExpressionNode::new(
                        cond_exp.content.clone(),
                        cond_exp.is_static,
                        cond_exp.loc.clone(),
                    );
                    let condition = Box::new_in(cond_node, ctx.allocator);
                    let positive = transform_children(ctx, &el.children);

                    let if_node = IfIRNode {
                        id: ctx.next_id(),
                        condition,
                        positive,
                        negative: None,
                        once: false,
                        parent: None,
                        anchor: None,
                    };

                    block
                        .operation
                        .push(OperationNode::If(Box::new_in(if_node, ctx.allocator)));
                }
            }
        }
        "for" => {
            // v-for
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(source_exp) = exp {
                    let source_node = SimpleExpressionNode::new(
                        source_exp.content.clone(),
                        source_exp.is_static,
                        source_exp.loc.clone(),
                    );
                    let source = Box::new_in(source_node, ctx.allocator);
                    let render = transform_children(ctx, &el.children);

                    let for_node = ForIRNode {
                        id: ctx.next_id(),
                        source,
                        value: None,
                        key: None,
                        index: None,
                        key_prop: None,
                        render,
                        once: false,
                        component: el.tag_type == ElementType::Component,
                        only_child: false,
                    };

                    block
                        .operation
                        .push(OperationNode::For(Box::new_in(for_node, ctx.allocator)));
                }
            }
        }
        "html" => {
            // v-html
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(val_exp) = exp {
                    let val_node = SimpleExpressionNode::new(
                        val_exp.content.clone(),
                        val_exp.is_static,
                        val_exp.loc.clone(),
                    );
                    let value = Box::new_in(val_node, ctx.allocator);
                    let set_html = SetHtmlIRNode {
                        element: element_id,
                        value,
                    };

                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetHtml(set_html));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        "text" => {
            // v-text
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(val_exp) = exp {
                    let mut values = Vec::new_in(ctx.allocator);
                    let val_node = SimpleExpressionNode::new(
                        val_exp.content.clone(),
                        val_exp.is_static,
                        val_exp.loc.clone(),
                    );
                    values.push(Box::new_in(val_node, ctx.allocator));

                    let set_text = SetTextIRNode {
                        element: element_id,
                        values,
                    };

                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetText(set_text));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        "show" => {
            // v-show - builtin directive
            let mut new_dir = DirectiveNode::new(ctx.allocator, dir.name.clone(), dir.loc.clone());
            if let Some(ref exp) = dir.exp {
                match exp {
                    ExpressionNode::Simple(s) => {
                        let node = SimpleExpressionNode::new(
                            s.content.clone(),
                            s.is_static,
                            s.loc.clone(),
                        );
                        new_dir.exp =
                            Some(ExpressionNode::Simple(Box::new_in(node, ctx.allocator)));
                    }
                    _ => {}
                }
            }

            let dir_node = DirectiveIRNode {
                element: element_id,
                dir: Box::new_in(new_dir, ctx.allocator),
                name: vize_carton::String::from("vShow"),
                builtin: true,
                tag: el.tag.clone(),
                input_type: get_static_attr(el, "type"),
            };

            block.operation.push(OperationNode::Directive(dir_node));
        }
        "model" => {
            // v-model - builtin directive
            let mut new_dir = DirectiveNode::new(ctx.allocator, dir.name.clone(), dir.loc.clone());
            if let Some(ref exp) = dir.exp {
                match exp {
                    ExpressionNode::Simple(s) => {
                        let node = SimpleExpressionNode::new(
                            s.content.clone(),
                            s.is_static,
                            s.loc.clone(),
                        );
                        new_dir.exp =
                            Some(ExpressionNode::Simple(Box::new_in(node, ctx.allocator)));
                    }
                    _ => {}
                }
            }
            if let Some(ref arg) = dir.arg {
                match arg {
                    ExpressionNode::Simple(s) => {
                        let node = SimpleExpressionNode::new(
                            s.content.clone(),
                            s.is_static,
                            s.loc.clone(),
                        );
                        new_dir.arg =
                            Some(ExpressionNode::Simple(Box::new_in(node, ctx.allocator)));
                    }
                    _ => {}
                }
            }
            for m in dir.modifiers.iter() {
                new_dir.modifiers.push(SimpleExpressionNode::new(
                    m.content.clone(),
                    m.is_static,
                    m.loc.clone(),
                ));
            }

            let dir_node = DirectiveIRNode {
                element: element_id,
                dir: Box::new_in(new_dir, ctx.allocator),
                name: vize_carton::String::from("model"),
                builtin: true,
                tag: el.tag.clone(),
                input_type: get_static_attr(el, "type"),
            };

            block.operation.push(OperationNode::Directive(dir_node));
        }
        _ => {
            // Custom directive - create a copy of the directive
            let new_dir = DirectiveNode::new(ctx.allocator, dir.name.clone(), dir.loc.clone());

            let dir_node = DirectiveIRNode {
                element: element_id,
                dir: Box::new_in(new_dir, ctx.allocator),
                name: dir.name.clone(),
                builtin: false,
                tag: el.tag.clone(),
                input_type: vize_carton::String::from(""),
            };

            block.operation.push(OperationNode::Directive(dir_node));
        }
    }
}

/// Get a static attribute value from an element
fn get_static_attr(el: &ElementNode<'_>, attr_name: &str) -> vize_carton::String {
    for prop in el.props.iter() {
        if let PropNode::Attribute(attr) = prop {
            if attr.name.as_str() == attr_name {
                if let Some(ref value) = attr.value {
                    return value.content.clone();
                }
            }
        }
    }
    vize_carton::String::from("")
}
