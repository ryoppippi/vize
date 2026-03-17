---
title: Oxlint Plugin
---

# Oxlint Plugin

`oxlint-plugin-vize` lets Oxlint execute Vize Patina diagnostics through Oxlint's JS plugin system. This is useful when you want Oxlint's Rust-native JavaScript and TypeScript rules together with Vize's Vue-specific diagnostics in a single run.

## Installation

If `oxlint` is already installed:

```bash
pnpm add -D oxlint-plugin-vize
```

If you are setting up Oxlint from scratch:

```bash
pnpm add -D oxlint oxlint-plugin-vize
```

`oxlint-plugin-vize` installs the matching Vize native binding for the current platform through optional dependencies, so no separate `@vizejs/native` install is required for consumers.

## Basic Usage

Enable Oxlint's built-in `vue` plugin and load the Vize bridge as a JS plugin:

```json
{
  "plugins": ["vue"],
  "jsPlugins": ["oxlint-plugin-vize"],
  "rules": {
    "vize/vue/require-v-for-key": "error",
    "vize/vue/no-v-html": "warn",
    "no-console": "warn"
  }
}
```

This keeps Oxlint core rules like `no-console`, while adding Vize's Vue diagnostics under the `vize/vue/*` namespace.

## Settings

Patina settings are passed through `settings.vize`:

```json
{
  "settings": {
    "vize": {
      "locale": "ja",
      "helpLevel": "short"
    }
  }
}
```

- `locale` controls the diagnostic language.
- `helpLevel` accepts `"full"`, `"short"`, or `"none"`.
- `showHelp` is still accepted for backward compatibility, but `helpLevel` is the preferred setting.

For compatibility with older configs, `settings.patina` is still accepted, but `settings.vize` is the canonical key.

## How It Works

- The first enabled Vize rule on a file runs a native Patina pass for that rule.
- If a second Vize rule is enabled for the same file, the bridge upgrades to one shared full-file Patina pass and reuses the result for the remaining rules.
- Native bindings are resolved by platform package first, then by the workspace `@vizejs/native` package during local development.

## Current Limitations

- Oxlint JS plugins currently rely on the extracted Vue script program. Files without `<script>` or `<script setup>` do not invoke the plugin yet.
- Oxlint JS plugins only accept ranges inside the extracted Vue script program. For template diagnostics, Vize inlines the original SFC block and `line:column` into the summary, while the fallback formatter anchor still points at the script block.

## Local Development in This Repository

If you are working inside the Vize repository itself:

```bash
vp install
vp run --filter './npm/vize-native' build
vp run --filter './npm/oxlint-plugin-vize' build
```

The package build uses `vp pack`, targets Node 24+, and follows the repository's `.node-version`.
