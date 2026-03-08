import * as fs from "node:fs";
import * as path from "node:path";
import { pathToFileURL } from "node:url";
import { execFileSync } from "node:child_process";
import { transform } from "oxc-transform";
import type {
  VizeConfig,
  LoadConfigOptions,
  UserConfigExport,
  ConfigEnv,
  GlobalTypesConfig,
  GlobalTypeDeclaration,
} from "./types/index.js";

const CONFIG_FILE_NAMES = [
  "vize.config.pkl",
  "vize.config.ts",
  "vize.config.js",
  "vize.config.mjs",
  "vize.config.json",
];

const DEFAULT_CONFIG_ENV: ConfigEnv = {
  mode: "development",
  command: "serve",
};

/**
 * Define a Vize configuration with type checking.
 * Accepts a plain object or a function that receives ConfigEnv.
 */
export function defineConfig(config: UserConfigExport): UserConfigExport {
  return config;
}

/**
 * Load vize.config file from the specified directory
 */
export async function loadConfig(
  root: string,
  options: LoadConfigOptions = {},
): Promise<VizeConfig | null> {
  const { mode = "root", configFile, env } = options;

  if (mode === "none") {
    return null;
  }

  // Custom config file path
  if (configFile) {
    const absolutePath = path.isAbsolute(configFile) ? configFile : path.resolve(root, configFile);
    if (fs.existsSync(absolutePath)) {
      return loadConfigFile(absolutePath, env);
    }
    return null;
  }

  // Search for config file
  if (mode === "auto") {
    return loadConfigFromDirAuto(root, env);
  }

  // mode === "root"
  return loadConfigFromDir(root, env);
}

/**
 * Try loading config from each supported file name in a directory.
 * Falls back to next format if pkl fails (e.g. pkl CLI not installed).
 */
async function loadConfigFromDir(dir: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  for (const name of CONFIG_FILE_NAMES) {
    const filePath = path.join(dir, name);
    if (fs.existsSync(filePath)) {
      const config = await loadConfigFile(filePath, env);
      if (config !== null) {
        return config;
      }
    }
  }
  return null;
}

/**
 * Search from cwd upward until finding a loadable config file.
 */
async function loadConfigFromDirAuto(startDir: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  let currentDir = path.resolve(startDir);
  const root = path.parse(currentDir).root;

  while (currentDir !== root) {
    const config = await loadConfigFromDir(currentDir, env);
    if (config !== null) {
      return config;
    }
    currentDir = path.dirname(currentDir);
  }

  return null;
}

/**
 * Load and evaluate a config file
 */
async function loadConfigFile(filePath: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  const ext = path.extname(filePath);

  if (ext === ".pkl") {
    return loadPklConfig(filePath);
  }

  if (ext === ".json") {
    const content = fs.readFileSync(filePath, "utf-8");
    return JSON.parse(content);
  }

  if (ext === ".ts") {
    return loadTypeScriptConfig(filePath, env);
  }

  // .js, .mjs - ESM
  return loadESMConfig(filePath, env);
}

/**
 * Find the pkl binary from @pkl-community/pkl or PATH
 */
function findPklBinary(): string | null {
  // Try @pkl-community/pkl package first
  try {
    const pklPkgPath = import.meta.resolve?.("@pkl-community/pkl");
    if (pklPkgPath) {
      const pklDir = path.dirname(new URL(pklPkgPath).pathname);
      const pklBin = path.join(pklDir, "pkl");
      if (fs.existsSync(pklBin)) {
        return pklBin;
      }
    }
  } catch {
    // package not installed
  }

  // Try PATH
  try {
    execFileSync("pkl", ["--version"], { stdio: "ignore" });
    return "pkl";
  } catch {
    // pkl not on PATH
  }

  return null;
}

/**
 * Load Pkl config file by evaluating it to JSON
 */
function loadPklConfig(filePath: string): VizeConfig | null {
  const pklBin = findPklBinary();
  if (!pklBin) {
    console.warn(
      "[vize] pkl CLI not found. Install @pkl-community/pkl or add pkl to PATH. " +
        "Falling back to next config format.",
    );
    return null;
  }

  try {
    const output = execFileSync(pklBin, ["eval", "-f", "json", filePath], {
      encoding: "utf-8",
      timeout: 30_000,
    });
    return JSON.parse(output);
  } catch (e) {
    console.warn(`[vize] Failed to evaluate ${filePath}:`, e);
    return null;
  }
}

/**
 * Resolve a UserConfigExport to a VizeConfig
 */
async function resolveConfigExport(
  exported: UserConfigExport,
  env?: ConfigEnv,
): Promise<VizeConfig> {
  if (typeof exported === "function") {
    return exported(env ?? DEFAULT_CONFIG_ENV);
  }
  return exported;
}

/**
 * Load TypeScript config file using oxc-transform
 */
async function loadTypeScriptConfig(filePath: string, env?: ConfigEnv): Promise<VizeConfig> {
  const source = fs.readFileSync(filePath, "utf-8");
  const result = transform(filePath, source, {
    typescript: {
      onlyRemoveTypeImports: true,
    },
  });

  const code = result.code;

  // Write to temp file and import (use Date.now() to avoid race conditions)
  const tempFile = filePath.replace(/\.ts$/, `.temp.${Date.now()}.mjs`);
  fs.writeFileSync(tempFile, code);

  try {
    const fileUrl = pathToFileURL(tempFile).href;
    const module = await import(fileUrl);
    const exported: UserConfigExport = module.default || module;
    return resolveConfigExport(exported, env);
  } finally {
    fs.unlinkSync(tempFile);
  }
}

/**
 * Load ESM config file
 */
async function loadESMConfig(filePath: string, env?: ConfigEnv): Promise<VizeConfig> {
  const fileUrl = pathToFileURL(filePath).href;
  const module = await import(fileUrl);
  const exported: UserConfigExport = module.default || module;
  return resolveConfigExport(exported, env);
}

/**
 * Normalize GlobalTypesConfig shorthand strings to GlobalTypeDeclaration objects
 */
export function normalizeGlobalTypes(
  config: GlobalTypesConfig,
): Record<string, GlobalTypeDeclaration> {
  const resolvedConfig =
    "types" in config &&
    typeof config.types === "object" &&
    config.types !== null &&
    !Array.isArray(config.types)
      ? config.types
      : config;

  const result: Record<string, GlobalTypeDeclaration> = {};
  for (const [key, value] of Object.entries(resolvedConfig)) {
    if (typeof value === "string") {
      result[key] = { type: value };
    } else {
      result[key] = value;
    }
  }
  return result;
}
