//! Production Croquis facts projected from their registered producers.
//! The shared descriptor orchestrator remains the authority for split scripts,
//! Options API, scopes and offsets; this boundary adds no semantic classifier.
#![expect(clippy::disallowed_types, reason = "JSON and N-API own std strings")]
#![expect(clippy::disallowed_methods, reason = "JSON and N-API own std strings")]
#![expect(clippy::disallowed_macros, reason = "JSON values use serde macros")]

mod components;
mod flow;
mod reactivity;
#[cfg(test)]
mod tests;

use super::{document::PluginDocument, error::HostError};
use serde_json::{Map, Value, json};
use std::sync::{Mutex, OnceLock};
use vize_atelier_sfc::croquis::{SfcCroquisOptions, analyze_sfc_descriptor};
use vize_croquis::sfc::{SfcParseOptions, parse_sfc};
use vize_croquis::{
    Croquis,
    facts::{
        BindingKey, Bindings, CROQUIS_FACTS, ComponentUsages, FactConsumer, FactGroup, FactManager,
        ProvideInject, RaceConditions, Reactivity, UndefinedRefs,
    },
};
use vize_davinci::fact::Demand;
use vize_s0::Allocator;

pub const GROUPS: &[&str] = &[
    Bindings::NAME,
    UndefinedRefs::NAME,
    ComponentUsages::NAME,
    Reactivity::NAME,
    ProvideInject::NAME,
    RaceConditions::NAME,
];

pub struct ProductionDocument {
    croquis: OnceLock<Result<Croquis, HostError>>,
    manager: Mutex<FactManager<'static, Croquis>>,
}
impl Default for ProductionDocument {
    fn default() -> Self {
        Self {
            croquis: OnceLock::new(),
            manager: Mutex::new(FactManager::new(&CROQUIS_FACTS)),
        }
    }
}
impl core::fmt::Debug for ProductionDocument {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ProductionDocument")
            .field("initialized", &self.croquis.get().is_some())
            .finish()
    }
}
impl ProductionDocument {
    pub fn get(&self, source: &str) -> Result<&Croquis, HostError> {
        self.croquis
            .get_or_init(|| {
                let descriptor = parse_sfc(source, SfcParseOptions::default())
                    .map_err(|error| HostError::Split(error.message.to_string()))?;
                for script in descriptor
                    .script
                    .iter()
                    .chain(descriptor.script_setup.iter())
                {
                    if script
                        .lang
                        .as_deref()
                        .is_some_and(|lang| !matches!(lang, "js" | "ts" | "jsx" | "tsx"))
                    {
                        return Err(HostError::Split(
                            "production JS plugin facts require a JavaScript or TypeScript script"
                                .into(),
                        ));
                    }
                }
                let allocator = Allocator::new();
                let root = descriptor.template.as_ref().map(|template| {
                    vize_atelier_core::parser::parse(&allocator, template.content.as_ref()).0
                });
                Ok(analyze_sfc_descriptor(
                    &descriptor,
                    root.as_ref(),
                    SfcCroquisOptions::full(),
                ))
            })
            .as_ref()
            .map_err(Clone::clone)
    }
}

struct PluginFacts;
impl FactConsumer for PluginFacts {
    const NAME: &'static str = "js-plugin-production-facts";
    const DEMAND: Demand = Demand::NONE
        .with(Bindings::ID)
        .with(UndefinedRefs::ID)
        .with(ComponentUsages::ID)
        .with(Reactivity::ID)
        .with(ProvideInject::ID)
        .with(RaceConditions::ID);
}

pub fn project(
    document: &PluginDocument,
    names: &[String],
) -> Result<Map<String, Value>, HostError> {
    let mut output = Map::new();
    let mut demand = Demand::NONE;
    for name in names {
        if let Some(entry) = CROQUIS_FACTS
            .producers()
            .iter()
            .find(|entry| entry.desc.name == name)
        {
            demand = demand.with(entry.desc.id);
        }
    }
    if demand.is_empty() {
        return Ok(output);
    }
    let croquis = document.production.get(&document.source)?;
    let mut manager = document
        .production
        .manager
        .lock()
        .map_err(|_| HostError::Split("production fact manager lock poisoned".into()))?;
    manager.compute(croquis, demand).map_err(|error| {
        HostError::Split(format!("production fact computation failed: {error:?}"))
    })?;
    let view = manager.view::<PluginFacts>();
    macro_rules! group {
        ($group:ty, $project:expr) => {
            if demand.contains(<$group>::ID) {
                let table = view.get::<$group>().map_err(|error| {
                    HostError::Split(format!("production fact read failed: {error:?}"))
                })?;
                let rows: Vec<Value> = table.iter().map($project).collect();
                output.insert(<$group>::NAME.into(), Value::Array(rows));
            }
        };
    }
    group!(Bindings, |(key, value)| match key {
        BindingKey::ScriptSetup => json!(["@script-setup", {"scriptSetup": true}]),
        BindingKey::Name(name) =>
            json!([name.as_str(), {"kind": value.kind, "span": value.span, "propKey": value.prop_key.as_deref()}]),
    });
    group!(
        UndefinedRefs,
        |(key, value)| json!([key, {"name": value.name.as_str(), "offset": value.offset, "context": value.context.as_str()}])
    );
    group!(ComponentUsages, components::project);
    group!(Reactivity, reactivity::project);
    group!(ProvideInject, flow::provide);
    group!(RaceConditions, flow::race);
    Ok(output)
}
