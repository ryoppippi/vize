# oxlint-plugin-vize

Oxlint JS plugin bridge for Vize Patina.

This package lets Oxlint execute Patina through Vize's native binding while still using Oxlint's JS plugin model and rule configuration.

## Performance

The bridge is optimized around Oxlint's per-rule execution model:

- The first enabled Patina rule on a file runs native linting for that rule only.
- If a second Patina rule is encountered on the same file, the bridge upgrades to one shared full-file Patina pass and reuses that result for the remaining Patina rules.
- File contents and rule results are cached per file and locale for the lifetime of the Oxlint process.

## Installation

`oxlint-plugin-vize` targets Node 24+. In this repository, Vite+ reads `.node-version` for you, so the usual setup is:

```bash
vp install
vp run --filter './npm/vize-native' build
vp run --filter './npm/oxlint-plugin-vize' build
```

If `oxlint` is already installed in your project, this is enough:

```bash
pnpm add -D oxlint-plugin-vize
```

If you are setting up Oxlint from scratch, install both packages:

```bash
pnpm add -D oxlint oxlint-plugin-vize
```

`oxlint-plugin-vize` pulls the appropriate Vize native binding for the current platform through optional dependencies, so no separate `@vizejs/native` install is required for published builds.

## Usage

Enable Oxlint's built-in `vue` plugin as well as this JS plugin:

```json
{
  "plugins": ["vue"],
  "jsPlugins": ["oxlint-plugin-vize"],
  "rules": {
    "vize/vue/require-v-for-key": "error",
    "vize/vue/no-v-html": "warn"
  }
}
```

You can pass Patina settings through `settings.vize`:

```json
{
  "settings": {
    "vize": {
      "locale": "ja",
      "showHelp": false
    }
  }
}
```

`showHelp` defaults to `true`. Set it to `false` when you want a denser terminal view without the long remediation block.

## Current limitations

- Oxlint JS plugins currently rely on the extracted Vue script program. Files without `<script>` or `<script setup>` do not invoke the plugin yet.
- Oxlint JS plugins only accept ranges inside the extracted Vue script program. For template diagnostics, the real SFC `line/column` is surfaced in the message, while the fallback formatter anchor still points at the script block.
