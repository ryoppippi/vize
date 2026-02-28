//! Diagnostics generation and reporting for cross-file reactivity analysis.

use super::analyzer::CrossFileReactivityAnalyzer;
use super::types::*;
use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use vize_carton::CompactString;

impl<'a> CrossFileReactivityAnalyzer<'a> {
    /// Generate diagnostics from detected issues.
    pub(super) fn generate_diagnostics(&self) -> Vec<CrossFileDiagnostic> {
        let mut diagnostics = Vec::new();

        for issue in &self.issues {
            let diag = match &issue.kind {
                CrossFileReactivityIssueKind::ComposableReturnDestructured {
                    composable_name,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: composable_name.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("toRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring {} return loses reactivity for: {}",
                        composable_name,
                        destructured_props.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "const result = {}(); then access result.prop or use toRefs(result)",
                    composable_name
                )),

                CrossFileReactivityIssueKind::InjectValueDestructured {
                    key,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: key.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("toRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring inject('{}') loses reactivity for: {}",
                        key,
                        destructured_props.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "const injected = inject('{}'); access injected.prop directly",
                    key
                )),

                CrossFileReactivityIssueKind::StoreDestructured {
                    store_name,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: store_name.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("storeToRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring {} loses reactivity for state/getters",
                        store_name
                    ),
                )
                .with_suggestion(format!(
                    "const {{ ... }} = storeToRefs({})",
                    store_name
                )),

                CrossFileReactivityIssueKind::PropsDestructured { destructured_props } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                            source_name: CompactString::new("props"),
                            destructured_keys: destructured_props.clone(),
                            suggestion: CompactString::new("toRefs"),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!(
                            "Destructuring props: {} (Vue compiler handles this, but explicit toRefs is clearer)",
                            destructured_props.join(", ")
                        ),
                    )
                    .with_suggestion("const { ... } = toRefs(props) for explicit reactivity")
                }

                CrossFileReactivityIssueKind::NonReactiveProvide { key } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::ReactivityOutsideSetup {
                            api_name: CompactString::new("provide"),
                            context_description: CompactString::new("non-reactive value"),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!(
                            "provide('{}') value is not reactive - consumers won't see updates",
                            key
                        ),
                    )
                    .with_suggestion("provide('key', ref(value)) or provide('key', reactive({...}))")
                }

                CrossFileReactivityIssueKind::CircularReactiveDependency { cycle } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::CircularReactiveDependency {
                            cycle: cycle.clone(),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!("Circular reactive dependency: {} \u{2192} ...", cycle.join(" \u{2192} ")),
                    )
                    .with_suggestion("Break the cycle by using computed() or reorganizing data flow")
                }

                _ => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::HydrationMismatchRisk {
                        reason: CompactString::new("cross-file reactivity issue"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!("{:?}", issue.kind),
                ),
            };

            diagnostics.push(diag);
        }

        diagnostics
    }

    /// Generate a markdown report of cross-file reactivity flows.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Cross-File Reactivity Report\n\n");

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "- **Tracked Reactive Values**: {}\n",
            self.reactive_values.len()
        ));
        md.push_str(&format!("- **Cross-File Flows**: {}\n", self.flows.len()));
        md.push_str(&format!("- **Issues Detected**: {}\n\n", self.issues.len()));

        // Flows
        if !self.flows.is_empty() {
            md.push_str("## Reactivity Flows\n\n");
            md.push_str("```\n");

            for flow in &self.flows {
                let status = if flow.preserved {
                    "\u{2713}"
                } else {
                    "\u{2717}"
                };
                let flow_type = match flow.flow_kind {
                    ReactivityFlowKind::ComposableExport => "composable",
                    ReactivityFlowKind::ProvideInject => "provide/inject",
                    ReactivityFlowKind::PropsFlow => "props",
                    ReactivityFlowKind::StoreFlow => "store",
                    ReactivityFlowKind::ModuleImport => "import",
                };

                md.push_str(&format!(
                    "{} [{}] {} \u{2192} {}\n",
                    status, flow_type, flow.source.name, flow.target.name
                ));

                if let Some(ref reason) = flow.loss_reason {
                    md.push_str(&format!("   \u{2514}\u{2500} Loss: {:?}\n", reason));
                }
            }

            md.push_str("```\n\n");
        }

        // Issues
        if !self.issues.is_empty() {
            md.push_str("## Issues\n\n");

            for issue in &self.issues {
                let icon = match issue.severity {
                    DiagnosticSeverity::Error => "\u{274c}",
                    DiagnosticSeverity::Warning => "\u{26a0}\u{fe0f}",
                    DiagnosticSeverity::Info => "\u{2139}\u{fe0f}",
                    DiagnosticSeverity::Hint => "\u{1f4a1}",
                };

                md.push_str(&format!("### {} {:?}\n\n", icon, issue.kind));
                md.push_str(&format!("- **File**: {:?}\n", issue.file_id));
                md.push_str(&format!("- **Offset**: {}\n", issue.offset));
                if let Some(related) = issue.related_file {
                    md.push_str(&format!("- **Related File**: {:?}\n", related));
                }
                md.push('\n');
            }
        }

        md
    }
}
