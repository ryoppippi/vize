# Oxlint + Vize Example

This is the smallest runnable example of Vize Patina executed through `oxlint-plugin-vize`.

## Prerequisites

This example uses the locally built Vize Oxlint plugin bundle and `@vizejs/native`. It assumes Node 24+ and the repository's `.node-version`, and the example scripts rebuild dependencies automatically. If you want to build them up front from the repository root:

```bash
vp install
vp run --filter './npm/vize-native' build
vp run --filter './npm/oxlint-plugin-vize' build
```

## Run

```bash
vp run --filter './examples/oxlint-vize' lint
```

This command intentionally exits non-zero because it includes `src/HasPatinaErrors.vue`.
It mixes:

- Oxlint core diagnostics from `no-console`
- Patina diagnostics from `oxlint-plugin-vize`
- The `stylish` formatter so the default code frame does not dominate the output
- `settings.vize.helpLevel: "none"` so the long Patina remediation block stays hidden by default

If you want to check `no-unused-vars` specifically:

```bash
vp run --filter './examples/oxlint-vize' lint:unused-vars-probe
```

Current observed behavior in this repository: that command reports `0` findings on `.vue` even though `src/UnusedVarProbe.vue` contains an unused binding. I verified separately that the same `no-unused-vars` rule does fire on a plain `.ts` file, so this is an Oxlint/Vue behavior to be aware of rather than a Patina bridge conflict.

If you only want the success path:

```bash
vp run --filter './examples/oxlint-vize' lint:clean
```

If you want JSON output from the same command:

```bash
vp run --filter './examples/oxlint-vize' lint:json
```

If you want the long Patina `Help:` block as well:

```bash
vp run --filter './examples/oxlint-vize' lint:with-help
```

If you want the raw direct CLI path, that also works once the plugin has been built:

```bash
vp exec oxlint -c .oxlintrc.json -f stylish src
```

## Notes

- Oxlint's built-in `vue` plugin is enabled through `"plugins": ["vue"]`.
- Oxlint's built-in `no-console` rule is enabled so the example shows native Oxlint output mixed with Patina output in one run.
- The default example commands use `-f stylish` because Oxlint's default formatter prints a large code frame for every finding, while `stylish` keeps the Patina message body intact.
- `settings.vize.helpLevel` controls remediation density. The example keeps it at `"none"` by default and exposes `lint:with-help` for the verbose view.
- A dedicated `lint:unused-vars-probe` command is included because `no-unused-vars` currently does not emit on the example `.vue` SFC, even without the Patina plugin.
- The Vize Oxlint plugin is loaded from `../../npm/oxlint-plugin-vize/dist/index.mjs`.
- The plugin starts with a single-rule native Patina run and only upgrades to a shared full-file pass when multiple Patina rules are enabled for the same file.
- Patina help text is normalized to plain text so terminal output stays readable even without Markdown rendering.
- Because of the current Oxlint JS Plugin limitation, the example `.vue` files include `<script setup>`.
- `src/HasPatinaErrors.vue` is the failing mixed-output sample, `src/Clean.vue` is the clean sample, and `src/UnusedVarProbe.vue` is the `no-unused-vars` probe.
