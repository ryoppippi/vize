//! Code generation context that tracks state during Vapor code emission.

use vize_carton::{cstr, FxHashMap, FxHashSet, String, ToCompactString};

/// For-loop scope entry
#[derive(Debug, Clone)]
pub(crate) struct ForScope {
    /// Value alias (e.g., "item") -> "_for_item{depth}"
    pub(crate) value_alias: Option<String>,
    /// Key alias (e.g., "index" or "key") -> "_for_key{depth}"
    pub(crate) key_alias: Option<String>,
    /// Index alias -> unused in current vapor
    pub(crate) index_alias: Option<String>,
    /// Depth of for nesting (0-based)
    pub(crate) depth: usize,
}

/// Generate context
pub(crate) struct GenerateContext<'a> {
    pub(crate) code: String,
    indent_level: u32,
    #[allow(dead_code)]
    pub(crate) element_template_map: &'a FxHashMap<usize, usize>,
    temp_count: usize,
    /// Used helpers for import generation
    pub(crate) used_helpers: FxHashSet<&'static str>,
    /// Events that need delegation (event names)
    pub(crate) delegate_events: FxHashSet<String>,
    /// Text node references (element_id -> text_node_var)
    pub(crate) text_nodes: FxHashMap<usize, String>,
    /// Whether currently inside a non-root block (v-if, v-for)
    pub(crate) is_fragment: bool,
    /// For-loop scope stack
    pub(crate) for_scopes: std::vec::Vec<ForScope>,
}

impl<'a> GenerateContext<'a> {
    pub(crate) fn new(element_template_map: &'a FxHashMap<usize, usize>) -> Self {
        Self {
            code: String::with_capacity(4096),
            indent_level: 0,
            element_template_map,
            temp_count: 0,
            used_helpers: FxHashSet::default(),
            delegate_events: FxHashSet::default(),
            text_nodes: FxHashMap::default(),
            is_fragment: false,
            for_scopes: std::vec::Vec::new(),
        }
    }

    /// Resolve an expression, replacing for-loop aliases with _for_item/key references
    pub(crate) fn resolve_expression(&self, expr: &str) -> String {
        // Check for-loop scopes (innermost first)
        for scope in self.for_scopes.iter().rev() {
            if let Some(ref value_alias) = scope.value_alias {
                let for_var = cstr!("_for_item{}", scope.depth);
                // Check if expression starts with or is the value alias
                if expr == value_alias.as_str() {
                    return cstr!("{}.value", for_var);
                }
                // Check member expression: "item.xxx"
                let prefix = [value_alias.as_str(), "."].concat();
                if expr.starts_with(&prefix) {
                    return cstr!("{}.value.{}", for_var, &expr[prefix.len()..]);
                }
            }
            if let Some(ref key_alias) = scope.key_alias {
                let for_key_var = cstr!("_for_key{}", scope.depth);
                if expr == key_alias.as_str() {
                    return cstr!("{}.value", for_key_var);
                }
                let prefix = [key_alias.as_str(), "."].concat();
                if expr.starts_with(&prefix) {
                    return cstr!("{}.value.{}", for_key_var, &expr[prefix.len()..]);
                }
            }
        }
        // Not a for-loop variable, use _ctx prefix
        cstr!("_ctx.{}", expr)
    }

    pub(crate) fn add_delegate_event(&mut self, event_name: &str) {
        self.delegate_events.insert(event_name.to_compact_string());
    }

    pub(crate) fn next_text_node(&mut self, element_id: usize) -> String {
        // Use element ID for text node variable name (x2 matches n2)
        let mut var_name = String::with_capacity(8);
        var_name.push('x');
        var_name.push_str(&element_id.to_compact_string());
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

    /// Push string to buffer (alias for `push`, compatible with `appends!`/`append!` macros)
    #[allow(dead_code)]
    pub(crate) fn push_str(&mut self, s: &str) {
        self.code.push_str(s);
    }

    /// Push formatted line (format_args! + newline with indentation)
    pub(crate) fn push_line_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        self.push_indent();
        use std::fmt::Write as _;
        self.write_fmt(args).unwrap();
        self.code.push('\n');
    }

    pub(crate) fn next_temp(&mut self) -> String {
        let name = cstr!("_t{}", self.temp_count);
        self.temp_count += 1;
        name
    }
}

impl std::fmt::Write for GenerateContext<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.code.push_str(s);
        Ok(())
    }
}
