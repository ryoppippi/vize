//! Style reads complete the demanded setup candidate relation.

use crate::SfcDescriptor;
use vize_croquis::Croquis;
use vize_croquis::drawer::extract_identifiers_checked;

pub(super) fn apply_style_reads(
    croquis: &mut Croquis,
    descriptor: &SfcDescriptor<'_>,
    derived: bool,
) {
    if descriptor
        .script_setup
        .as_ref()
        .is_some_and(|block| block.src.is_some())
        || descriptor.template.as_ref().is_some_and(|block| {
            block.src.is_some()
                || (!derived && block.lang.as_deref().is_some_and(|lang| lang != "html"))
        })
        || descriptor.styles.iter().any(|style| style.src.is_some())
    {
        // External or unsupported blocks can contain reads we cannot see.
        croquis.unused_bindings.clear();
        return;
    }
    for style in &descriptor.styles {
        let Some(ranges) =
            vize_croquis::sfc::__internal::checked_v_bind_expression_ranges(&style.content)
        else {
            croquis.unused_bindings.clear();
            return;
        };
        for range in ranges {
            let Some(expression) = style.content.get(range) else {
                continue;
            };
            let Some(reads) = extract_identifiers_checked(expression) else {
                croquis.unused_bindings.clear();
                return;
            };
            for name in reads {
                croquis
                    .unused_bindings
                    .retain(|candidate| candidate != &name);
            }
        }
    }
}
