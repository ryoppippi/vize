//! Remove proven template reads from the script's unused candidate relation.

use vize_carton::CompactString;

use super::Drawer;

impl Drawer {
    pub(super) fn read_setup_tag(&mut self, tag: &str) {
        if self.croquis.unused_bindings.is_empty() {
            return;
        }
        // Vue resolves Foo, foo-bar and Foo.Bar through the setup scope.
        let root = tag.split('.').next().unwrap_or(tag);
        let normalized = normalize(root);
        self.croquis
            .unused_bindings
            .retain(|name| normalize(name) != normalized);
    }

    pub(super) fn read_setup_directive(&mut self, directive: &str) {
        if !self.croquis.unused_bindings.is_empty() {
            let spelling = vize_carton::cstr!("v-{directive}");
            self.read_setup_tag(&spelling);
        }
    }
}

/// Vue's component/directive resolution ignores hyphens and case.
pub(crate) fn normalize(name: &str) -> CompactString {
    name.chars()
        .filter(|ch| *ch != '-')
        .flat_map(char::to_lowercase)
        .collect()
}
