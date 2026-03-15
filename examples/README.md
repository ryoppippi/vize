# Vize Examples

This directory contains runnable examples for trying Vize tools locally.

## Prerequisites

Build the repository from the project root before using the examples:

```bash
vp env install
vp install
vp run --workspace-root cli  # enable the `vize` CLI command
```

Or build directly with Cargo:

```bash
cargo build --release
```

---

## CLI Examples

`examples/cli/` contains sample Vue files for trying the CLI tools.

### Files

| File | Description |
|------|-------------|
| `src/App.vue` | Already-formatted Vue file |
| `src/Unformatted.vue` | Vue file that needs formatting |
| `src/HasErrors.vue` | Vue file that contains lint errors |

### Formatter (`vize fmt`)

```bash
# Check whether formatting is required
vize fmt examples/cli/src/*.vue --check

# Print formatted output without modifying files
vize fmt examples/cli/src/Unformatted.vue

# Write changes to files
vize fmt examples/cli/src/Unformatted.vue --write

# Use additional options
vize fmt examples/cli/src/*.vue --single-quote --no-semi --print-width 80
```

**Options**

| Option | Description | Default |
|--------|-------------|---------|
| `--check` | Exit with an error if changes are required | - |
| `--write`, `-w` | Write changes to files | - |
| `--single-quote` | Use single quotes | `false` |
| `--no-semi` | Omit semicolons | `false` |
| `--print-width` | Maximum line width | `100` |
| `--tab-width` | Indentation width | `2` |
| `--use-tabs` | Use tabs for indentation | `false` |

### Linter (`vize lint`)

```bash
# Show lint errors
vize lint examples/cli/src/*.vue

# Output JSON
vize lint examples/cli/src/HasErrors.vue --format json

# Limit the number of warnings
vize lint examples/cli/src/*.vue --max-warnings 5

# Show only the summary
vize lint examples/cli/src/*.vue --quiet
```

**Options**

| Option | Description | Default |
|--------|-------------|---------|
| `--format`, `-f` | Output format (`text` or `json`) | `text` |
| `--max-warnings` | Maximum allowed warning count | - |
| `--quiet`, `-q` | Show only the summary | `false` |
| `--fix` | Apply automatic fixes (not implemented yet) | `false` |

### LSP Server (`vize lsp`)

```bash
# Start over stdio (for editor integration)
vize lsp

# Specify a TCP port
vize lsp --port 3000

# Enable debug logging
vize lsp --debug
```

**Editor configuration example (VS Code)**

`.vscode/settings.json`

```json
{
  "vize.lsp.path": "/path/to/vize",
  "vize.lsp.args": ["lsp", "--debug"]
}
```

---

## Vite + Musea Example

`examples/vite-musea/` contains a sample component gallery built with Vite + Musea.

### Setup

```bash
cd examples/vite-musea
pnpm install
pnpm dev
```

### Usage

1. Start the dev server with `pnpm dev`.
2. Open `http://localhost:5173` in your browser.
3. Open `http://localhost:5173/__musea__` to view the component gallery.

### Files

| File | Description |
|------|-------------|
| `src/components/Button.vue` | Button component |
| `src/components/Button.art.vue` | Musea art file defining variants |
| `vite.config.ts` | Vite + Musea configuration |

### Writing Art Files

`.art.vue` files define component variants:

```vue
<art title="Button" component="./Button.vue" category="Components" status="ready">
  <variant name="Default" default>
    <Button>Default Button</Button>
  </variant>
  <variant name="Primary">
    <Button variant="primary">Primary Button</Button>
  </variant>
</art>

<script setup lang="ts">
import Button from "./Button.vue";
</script>
```

**`<art>` attributes**

| Attribute | Description |
|-----------|-------------|
| `title` | Component title (required) |
| `component` | Path to the target component |
| `category` | Category name |
| `status` | Status (`draft`, `ready`, `deprecated`) |

**`<variant>` attributes**

| Attribute | Description |
|-----------|-------------|
| `name` | Variant name (required) |
| `default` | Marks the default variant |
| `skip-vrt` | Skips the visual regression test |

---

## Oxlint + Patina Example

`examples/oxlint-patina/` contains the smallest runnable setup for executing Patina from Oxlint through `@vizejs/oxlint-plugin-patina`.

### Setup

Run this from the repository root:

```bash
vp install
vp run --filter './npm/vize-native' build
pnpm --filter @vizejs/oxlint-plugin-patina build
```

### Run

```bash
pnpm -C examples/oxlint-patina lint
```

This command intentionally exits non-zero because it includes `src/HasPatinaErrors.vue`. It mixes Oxlint core output with Patina output and uses the `stylish` formatter so the default code frame does not dominate the output. If you only want the success path:

```bash
pnpm -C examples/oxlint-patina lint:clean
```

If you want JSON output:

```bash
pnpm -C examples/oxlint-patina lint:json
```

To turn the long Patina `Help:` block back on:

```bash
pnpm -C examples/oxlint-patina lint:with-help
```

To probe `no-unused-vars` on a Vue SFC:

```bash
pnpm -C examples/oxlint-patina lint:unused-vars-probe
```

Current observed behavior in this repository: that probe reports `0` findings on `.vue`, even though the sample file contains an unused binding.

### Files

| File | Description |
|------|-------------|
| `.oxlintrc.json` | Oxlint config enabling `vue` and the Patina JS plugin |
| `.oxlintrc.unused-vars.json` | Dedicated probe config for `no-unused-vars` on a Vue SFC |
| `src/HasPatinaErrors.vue` | Sample SFC that intentionally triggers Patina diagnostics |
| `src/Clean.vue` | Clean success-case sample |
| `src/UnusedVarProbe.vue` | Probe file for current `no-unused-vars` behavior on `.vue` |
| `README.md` | Run instructions and current limitations |

---

## Troubleshooting

### `vize` Command Not Found

```bash
vp run --workspace-root cli

# Or run Cargo directly
cargo run --release -- fmt examples/cli/src/*.vue
```

### Native Binding Errors

When using the Musea plugin, `@vizejs/native` must be built:

```bash
vp run --workspace-root build:native
```
