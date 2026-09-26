//! Conservative, demand-only free reads for expressions without a retained AST.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use vize_carton::{CompactString, cstr};

/// Unknown syntax or direct eval cannot prove absence of a setup read.
pub(super) fn checked_reads(source: &str) -> Option<Vec<CompactString>> {
    let allocator = Allocator::default();
    let wrapped = cstr!("({source})");
    let expression = Parser::new(&allocator, &wrapped, SourceType::tsx()).parse();
    let parsed = if expression.panicked || !expression.diagnostics.is_empty() {
        // Inline event handlers may be statement bodies.
        Parser::new(&allocator, source, SourceType::tsx()).parse()
    } else {
        expression
    };
    if parsed.panicked || !parsed.diagnostics.is_empty() {
        return None;
    }
    let built = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .with_build_nodes(true)
        .build(&parsed.program);
    let semantic = &built.semantic;
    let scoping = semantic.scoping();
    if !built.diagnostics.is_empty()
        || scoping
            .scope_descendants_from_root()
            .any(|scope| scoping.scope_flags(scope).contains_direct_eval())
    {
        return None;
    }
    Some(
        scoping
            .root_unresolved_references_ids()
            .flatten()
            .filter_map(|id| {
                let reference = scoping.get_reference(id);
                let flags = reference.flags();
                (flags.is_read() || flags.is_type() || flags.is_value_as_type())
                    .then(|| CompactString::new(semantic.reference_name(reference)))
            })
            .collect(),
    )
}
