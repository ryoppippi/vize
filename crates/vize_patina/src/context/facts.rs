//! Per-artifact fact manager and analysis visibility.

use super::LintContext;
use vize_croquis::{
    Croquis,
    facts::{FactConsumer, FactView},
};

impl<'a> LintContext<'a> {
    /// Set semantic analysis.
    #[inline]
    pub fn set_analysis(&mut self, analysis: &'a Croquis) {
        self.analysis = Some(analysis);
        self.facts = Some(vize_croquis::facts::CroquisFacts::new(analysis));
    }

    /// Exclude selected rules from seeing semantic analysis in this pass.
    #[inline]
    pub fn set_analysis_excluded_rules(&mut self, rules: &'static [&'static str]) {
        self.analysis_excluded_rules = Some(rules);
    }

    /// Get semantic analysis (if available).
    #[inline]
    pub fn analysis(&self) -> Option<&Croquis> {
        if self
            .analysis_excluded_rules
            .is_some_and(|rules| rules.contains(&self.current_rule))
        {
            return None;
        }
        self.analysis
    }

    /// Check if semantic analysis is available.
    #[inline]
    pub fn has_analysis(&self) -> bool {
        self.analysis().is_some()
    }

    /// Read a consumer's declared groups, computed once for this artifact.
    pub fn facts<C: FactConsumer>(&mut self) -> Option<FactView<'_>> {
        self.analysis()?;
        self.facts.as_mut().map(|facts| facts.prepare::<C>())
    }
}
