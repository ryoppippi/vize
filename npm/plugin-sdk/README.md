# @vizejs/plugin-sdk

The authoring package for serialized JavaScript plugin visits. The package is
currently private while P6-7 integrates and verifies the remaining native hook
families. It does not advertise GA or sandboxing of arbitrary JavaScript.

## Lint rules and autofixes

```js
import { definePlugin, applyFixes } from "@vizejs/plugin-sdk";

const plugin = definePlugin({
  name: "team-rule",
  version: "1.0.0",
  visit: ["ui.bind"],
  demands: [],
  cacheInputs: [],
  rules: {
    check(ctx) {
      for (const node of ctx.nodes) {
        if (node.name === "disabled" && node.value === "true") {
          ctx.report(node, "Use a literal boolean attribute.", "disabled");
        }
      }
    },
  },
});
const output = native.lintWithPlugins(source, [plugin], {
  filename: "Button.vue",
  cache: true,
  validateDeterminism: true,
});
const fixed = applyFixes(source, output.fixes);
```

A report names a visited S2 node. Rust assigns its authored byte range; a fix
replaces that whole range. `applyFixes` handles UTF-8 bytes, preserves all other
source, deduplicates identical suggestions and rejects conflicting edits
without applying any. Fixes are suggestions; callers choose when to apply them
and rerun their compiler and linter after applying.

Rules are synchronous. Batch records, manifests and context values are frozen.
A static `demands` list controls which fact groups cross the boundary;
undeclared reads throw. `cacheInputs` must name every captured configuration or
ambient value, even when the list is empty. Changing source, filename, plugin
version, code, visits, demands, declared inputs or host build changes the key.

Caching always audits identical batches twice on a miss, even when
`validateDeterminism: false` is requested. Uncached runs audit when
`validateDeterminism: true` is set. Different diagnostics or fixes reject before
writing a cache entry. Cost fields include both calls; `jsNs` records JS call
time and `elapsedNs` includes host work. A cache hit has no JS call.

## Trust and current contract

Plugins execute synchronously on the calling Node thread. They are trusted
application code: a frozen batch and a determinism audit do not isolate the
process, filesystem, network or ambient globals. A malicious plugin can hide
ambient reads or block the thread. Install only code you trust. The current
static demands and node-owned ranges constrain data access and output shape,
not JavaScript authority.

The measured proxy handle remains available for spike compatibility; new
plugins use this package and serialized visits. `@vizejs/extension-sdk` is a
separate versioned WIT guest interface, not this package.
