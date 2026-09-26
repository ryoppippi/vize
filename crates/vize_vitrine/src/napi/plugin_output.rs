//! Batched JS formatter/output hooks after real template compilation.
#![expect(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "N-API boundary uses standard strings"
)]

use crate::{
    CompileResult, CompilerOptions,
    plugin_output::{self, CacheInput, PluginSpec},
};
use napi::Env;
use napi::bindgen_prelude::{Error, FunctionRef, Result, Status};
use napi_derive::napi;

#[napi(object, object_to_js = false)]
pub struct OutputPluginNapi {
    pub name: String,
    pub version: String,
    pub fingerprint: String,
    pub family: String,
    pub cache_inputs: Option<Vec<OutputPluginCacheInputNapi>>,
    /// One serialized batch in, one edit/addition array out.
    pub run: FunctionRef<String, String>,
}

#[napi(object)]
pub struct OutputPluginCacheInputNapi {
    pub name: String,
    pub value: String,
}

#[derive(Default)]
#[napi(object)]
pub struct OutputPluginOptionsNapi {
    pub cache: Option<bool>,
    pub cache_dir: Option<String>,
    /// On a miss, compare two validated outputs. Defaults to true.
    pub audit_determinism: Option<bool>,
}

#[napi(object)]
pub struct OutputPluginCostNapi {
    pub name: String,
    pub version: String,
    pub family: String,
    pub content_key: String,
    pub batch_bytes: u32,
    pub operations: u32,
    pub cached: bool,
    pub elapsed_ns: f64,
    pub js_ns: f64,
}

#[napi(object)]
pub struct OutputPluginsCompileResultNapi {
    pub result: CompileResult,
    pub plugins: Vec<OutputPluginCostNapi>,
}

/// Compile normally, then run checked, map-preserving output hooks.
#[napi(js_name = "compileWithOutputPlugins")]
pub fn compile_with_output_plugins(
    env: Env,
    template: String,
    plugins: Vec<OutputPluginNapi>,
    options: Option<CompilerOptions>,
    host_options: Option<OutputPluginOptionsNapi>,
) -> Result<OutputPluginsCompileResultNapi> {
    let options = options.unwrap_or_default();
    let config = serde_json::to_string(&options).map_err(invalid)?;
    let result = if options.output_mode.as_deref() == Some("vapor") {
        super::template::compile_vapor(template, Some(options))?
    } else {
        super::template::compile(template, Some(options))?
    };
    apply(env, result, plugins, host_options, &config)
}

/// Compose output hooks with any native compiler's checked result.
#[napi(js_name = "applyOutputPlugins")]
pub fn apply_output_plugins(
    env: Env,
    compiled: CompileResult,
    plugins: Vec<OutputPluginNapi>,
    options: Option<OutputPluginOptionsNapi>,
) -> Result<OutputPluginsCompileResultNapi> {
    apply(env, compiled, plugins, options, "apply-native-output@1")
}

fn apply(
    env: Env,
    mut result: CompileResult,
    plugins: Vec<OutputPluginNapi>,
    options: Option<OutputPluginOptionsNapi>,
    config: &str,
) -> Result<OutputPluginsCompileResultNapi> {
    let options = options.unwrap_or_default();
    let mut costs = Vec::with_capacity(plugins.len());
    for plugin in &plugins {
        let inputs = plugin.cache_inputs.as_ref().map(|inputs| {
            inputs
                .iter()
                .map(|input| CacheInput {
                    name: input.name.clone(),
                    value: input.value.clone(),
                })
                .collect::<Vec<_>>()
        });
        let spec = PluginSpec {
            name: &plugin.name,
            version: &plugin.version,
            fingerprint: &plugin.fingerprint,
            family: &plugin.family,
            inputs: inputs.as_deref(),
        };
        let callback = plugin.run.borrow_back(&env)?;
        let (next, cost) = plugin_output::run(
            &result,
            &spec,
            config,
            options.cache == Some(true),
            options.audit_determinism != Some(false),
            options.cache_dir.as_deref().map(std::path::Path::new),
            |batch| callback.call(batch).map_err(|e| e.to_string()),
        )
        .map_err(invalid)?;
        result = next;
        costs.push(OutputPluginCostNapi {
            name: cost.name,
            version: cost.version,
            family: cost.family,
            content_key: cost.content_key,
            batch_bytes: cost.batch_bytes,
            operations: cost.operations,
            cached: cost.cached,
            elapsed_ns: cost.elapsed_ns,
            js_ns: cost.js_ns,
        });
    }
    Ok(OutputPluginsCompileResultNapi {
        result,
        plugins: costs,
    })
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}
