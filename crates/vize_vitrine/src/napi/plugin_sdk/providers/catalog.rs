//! Validate provider ownership and compute the declared dependency closure.
use std::collections::{BTreeMap, BTreeSet};

use super::super::batch::{PluginSpec, validate_spec};
use super::super::error::HostError;
use super::super::facts::JS_VISIBLE;
use super::super::plugin_cache::validate_cache_inputs;
use super::{ProviderError, ProviderSpec, refusal};

pub struct ProviderCatalog {
    owners: BTreeMap<String, usize>,
    dependencies: Vec<Vec<usize>>,
}

impl ProviderCatalog {
    pub fn new(specs: &[ProviderSpec<'_>]) -> Result<Self, ProviderError> {
        let mut owners = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for (index, provider) in specs.iter().enumerate() {
            let name = provider.plugin.name;
            if name.is_empty()
                || name == "@vize"
                || name.starts_with("@vize/")
                || !identities.insert(name)
            {
                return Err(refusal(
                    name,
                    "provider identity is empty, reserved or duplicated",
                ));
            }
            if provider.plugin.version.is_empty() || provider.plugin.fingerprint.is_empty() {
                return Err(refusal(
                    name,
                    "version and code fingerprint must be nonempty",
                ));
            }
            validate_cache_inputs(name, provider.inputs_declared, provider.inputs)?;
            if provider
                .inputs
                .iter()
                .any(|input| input.name.starts_with("@vize/"))
            {
                return Err(refusal(
                    name,
                    "cacheInputs names beginning with `@vize/` are reserved",
                ));
            }
            if provider.provides.is_empty() {
                return Err(refusal(name, "provides must declare at least one group"));
            }
            let prefix = format!("{name}/");
            for group in provider.provides {
                if JS_VISIBLE.contains(&group.as_str())
                    || group.starts_with("@vize/")
                    || !group.starts_with(&prefix)
                    || group.len() == prefix.len()
                    || group.chars().any(char::is_whitespace)
                    || owners.insert(group.clone(), index).is_some()
                {
                    return Err(refusal(
                        name,
                        &format!("provided group `{group}` is reserved, unowned or duplicated"),
                    ));
                }
            }
        }
        let mut catalog = Self {
            owners,
            dependencies: Vec::with_capacity(specs.len()),
        };
        for provider in specs {
            catalog.validate_native(&provider.plugin)?;
            let mut dependencies = BTreeSet::new();
            for group in provider.plugin.demands {
                if let Some(owner) = catalog.owner(provider.plugin.name, group)? {
                    dependencies.insert(owner);
                }
            }
            catalog
                .dependencies
                .push(dependencies.into_iter().collect());
        }
        let mut state = vec![0_u8; specs.len()];
        let mut order = Vec::new();
        for index in 0..specs.len() {
            catalog.visit(index, &mut state, &mut order)?;
        }
        Ok(catalog)
    }

    pub fn demanded(&self, spec: &PluginSpec<'_>) -> Result<Vec<usize>, ProviderError> {
        self.validate_native(spec)?;
        let mut state = vec![0_u8; self.dependencies.len()];
        let mut order = Vec::new();
        for group in spec.demands {
            if let Some(index) = self.owner(spec.name, group)? {
                self.visit(index, &mut state, &mut order)?;
            }
        }
        Ok(order)
    }

    fn validate_native(&self, spec: &PluginSpec<'_>) -> Result<(), ProviderError> {
        let native: Vec<_> = spec
            .demands
            .iter()
            .filter(|name| JS_VISIBLE.contains(&name.as_str()))
            .cloned()
            .collect();
        validate_spec(&PluginSpec {
            demands: &native,
            ..*spec
        })?;
        Ok(())
    }

    fn owner(&self, plugin: &str, group: &str) -> Result<Option<usize>, ProviderError> {
        if JS_VISIBLE.contains(&group) {
            return Ok(None);
        }
        self.owners.get(group).copied().map(Some).ok_or_else(|| {
            HostError::UnknownFact {
                plugin: plugin.to_owned(),
                name: group.to_owned(),
                known: JS_VISIBLE,
            }
            .into()
        })
    }

    fn visit(
        &self,
        index: usize,
        state: &mut [u8],
        order: &mut Vec<usize>,
    ) -> Result<(), ProviderError> {
        match state.get(index).copied() {
            Some(2) => return Ok(()),
            Some(1) => return Err(refusal("host", "provider dependency cycle")),
            Some(0) => {}
            _ => return Err(refusal("host", "invalid provider dependency index")),
        }
        if let Some(mark) = state.get_mut(index) {
            *mark = 1;
        }
        if let Some(dependencies) = self.dependencies.get(index) {
            for dependency in dependencies {
                self.visit(*dependency, state, order)?;
            }
        }
        if let Some(mark) = state.get_mut(index) {
            *mark = 2;
        }
        order.push(index);
        Ok(())
    }
}
