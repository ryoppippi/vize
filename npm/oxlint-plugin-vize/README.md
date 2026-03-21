# oxlint-plugin-vize

Oxlint JS plugin bridge for Vize Patina.

This package lets Oxlint execute Patina through Vize's native binding while still using Oxlint's JS plugin model and rule configuration.

> [!IMPORTANT]
> As of March 21, 2026, `oxlint-plugin-vize` is not yet on npm. The first public release is planned as an alpha.
> Until [oxc-project/oxc#20465](https://github.com/oxc-project/oxc/issues/20465) lands, prefer `oxlint -f stylish` for terminal workflows and treat machine-readable / full-fidelity original-SFC reporting as best-effort.

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

Once the alpha release is published, install it with:

```bash
pnpm add -D oxlint oxlint-plugin-vize@alpha
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
      "preset": "essential",
      "helpLevel": "short"
    }
  }
}
```

- `preset` accepts `"happy-path"`, `"essential"`, `"opinionated"`, or `"nuxt"`.
- `preset` defaults to `"happy-path"`.
- Rules outside the selected preset stay quiet even if they are still listed in `rules`.
- `opinionated` is the preset that enables Vize's built-in script rules such as `vize/script/no-options-api`.
- `helpLevel` accepts `"full"`, `"short"`, or `"none"`.
- `helpLevel: "full"` only expands the Patina remediation text. It does not restore original-SFC formatter anchors or machine-readable range fidelity.
- `showHelp` is still accepted for backward compatibility, but `helpLevel` is the preferred setting.

For example, this keeps Oxlint focused on correctness-only diagnostics:

```json
{
  "settings": {
    "vize": {
      "preset": "essential",
      "helpLevel": "short"
    }
  },
  "rules": {
    "vize/vue/require-v-for-key": "error",
    "vize/vue/require-scoped-style": "error"
  }
}
```

In that config, `vize/vue/require-v-for-key` can report, while `vize/vue/require-scoped-style` stays silent because it belongs to the broader `happy-path` preset.

For day-to-day terminal runs, the recommended command today is:

```bash
pnpm exec oxlint -c .oxlintrc.json -f stylish src
```

`stylish` is currently the most usable compromise for mixed Oxlint + Vize output because the Patina summary can inline the original SFC location even though Oxlint still anchors JS plugin diagnostics to the extracted script program.

## Current limitations

- Oxlint JS plugins currently rely on the extracted Vue script program. Files without `<script>` or `<script setup>` do not invoke the plugin yet.
- Oxlint JS plugins only accept ranges inside the extracted Vue script program. For template diagnostics, Vize now inlines the original SFC block and `line:column` into the summary, while the formatter anchor still points at the script block.
- Because of [oxc-project/oxc#20465](https://github.com/oxc-project/oxc/issues/20465), formatter parity is not there yet. `stylish` is recommended for human-readable terminal output, while `json` and other machine-readable outputs are best treated as debugging aids for now.

## Alpha expectations

- The planned alpha release is meant for terminal-first workflows.
- The alpha is not yet a promise of precise original-SFC spans across every Oxlint formatter.
- Once Oxlint can preserve original Vue positions for JS plugins reliably, Vize can improve formatter parity and machine-readable reporting without relying on summary fallbacks.
