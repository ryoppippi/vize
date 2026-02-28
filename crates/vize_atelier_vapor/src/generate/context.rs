//! Code generation context that tracks state during Vapor code emission.

use vize_carton::FxHashMap;

/// Generate context
pub(crate) struct GenerateContext<'a> {
    pub(crate) code: String,
    indent_level: u32,
    #[allow(dead_code)]
    pub(crate) element_template_map: &'a FxHashMap<usize, usize>,
    temp_count: usize,
    /// Used helpers for import generation
    pub(crate) used_helpers: std::collections::HashSet<&'static str>,
    /// Events that need delegation (event names)
    pub(crate) delegate_events: std::collections::HashSet<String>,
    /// Text node references (element_id -> text_node_var)
    pub(crate) text_nodes: FxHashMap<usize, String>,
}

impl<'a> GenerateContext<'a> {
    pub(crate) fn new(element_template_map: &'a FxHashMap<usize, usize>) -> Self {
        Self {
            code: String::with_capacity(4096),
            indent_level: 0,
            element_template_map,
            temp_count: 0,
            used_helpers: std::collections::HashSet::new(),
            delegate_events: std::collections::HashSet::new(),
            text_nodes: FxHashMap::default(),
        }
    }

    pub(crate) fn add_delegate_event(&mut self, event_name: &str) {
        self.delegate_events.insert(event_name.to_string());
    }

    pub(crate) fn next_text_node(&mut self, element_id: usize) -> String {
        // Use element ID for text node variable name (x2 matches n2)
        let mut var_name = String::with_capacity(8);
        var_name.push('x');
        var_name.push_str(&element_id.to_string());
        self.text_nodes.insert(element_id, var_name.clone());
        var_name
    }

    pub(crate) fn use_helper(&mut self, name: &'static str) {
        self.used_helpers.insert(name);
    }

    pub(crate) fn push(&mut self, s: &str) {
        self.code.push_str(s);
    }

    pub(crate) fn push_line(&mut self, s: &str) {
        self.push_indent();
        self.code.push_str(s);
        self.code.push('\n');
    }

    pub(crate) fn push_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.code.push_str("  ");
        }
    }

    pub(crate) fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub(crate) fn deindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    pub(crate) fn next_temp(&mut self) -> String {
        let name = format!("_t{}", self.temp_count);
        self.temp_count += 1;
        name
    }
}
