# Vize

High-performance Vue.js toolchain in Rust.

## Installation

```bash
# Install from npm
npm install -g vize

# Or install from GitHub
npm install -g github:ubugeeei/vize
```

## Usage

```bash
vize [COMMAND] [OPTIONS]
```

| Command | Description                            |
| ------- | -------------------------------------- |
| `build` | Compile Vue SFC files (default)        |
| `fmt`   | Format Vue SFC files                   |
| `lint`  | Lint Vue SFC files                     |
| `check` | Type check Vue SFC files               |
| `musea` | Start component gallery server         |
| `lsp`   | Start Language Server Protocol server  |

```bash
vize --help           # Show help
vize <command> --help # Show command-specific help
```

## Examples

```bash
vize                              # Compile ./**/*.vue to ./dist
vize build src/**/*.vue -o out    # Custom input/output
vize build --ssr                  # SSR mode
vize fmt --check                  # Check formatting
vize lint --fix                   # Auto-fix lint issues
vize check --strict               # Strict type checking
```

## Configuration

Vize CLI and IDE settings can be shared from `vize.config.pkl`.

```pkl
amends "node_modules/vize/pkl/VizeConfig.pkl"

formatter {
  singleQuote = true
  printWidth = 88
}

typeChecker {
  globalsFile = "./types/global.d.ts"
}

languageServer {
  completion = false
}
```

The npm package ships the default Pkl modules in `node_modules/vize/pkl`, so users can extend them directly with `amends`.
These Pkl modules are the primary distributed schema/defaults; the generated JSON Schema is secondary compatibility output.

## Alternative Installation

If npm installation fails, you can install via Cargo:

```bash
cargo install vize
```

## License

MIT
