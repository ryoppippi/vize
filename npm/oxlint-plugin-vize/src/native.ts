import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

import type { PatinaBinding } from "./model.js";

const require = createRequire(import.meta.url);
const FALLBACK_BINDING_PACKAGE = "@vizejs/native";

let bindingCache: PatinaBinding | null = null;

function isMusl(): boolean {
  const report = process.report?.getReport();
  if (typeof report === "object" && report !== null && "header" in report) {
    const header = (report as { header: { glibcVersionRuntime?: string } }).header;
    return !header.glibcVersionRuntime;
  }

  try {
    const lddPath = require("node:child_process").execSync("which ldd").toString().trim();
    return readFileSync(lddPath, "utf8").includes("musl");
  } catch {
    return true;
  }
}

function getBindingPackageName(): string {
  const { arch, platform } = process;

  switch (platform) {
    case "darwin":
      switch (arch) {
        case "arm64":
          return "@vizejs/native-darwin-arm64";
        case "x64":
          return "@vizejs/native-darwin-x64";
        default:
          throw new Error(`Unsupported architecture on macOS: ${arch}`);
      }
    case "linux":
      switch (arch) {
        case "arm64":
          return isMusl() ? "@vizejs/native-linux-arm64-musl" : "@vizejs/native-linux-arm64-gnu";
        case "x64":
          return isMusl() ? "@vizejs/native-linux-x64-musl" : "@vizejs/native-linux-x64-gnu";
        default:
          throw new Error(`Unsupported architecture on Linux: ${arch}`);
      }
    case "win32":
      switch (arch) {
        case "arm64":
          return "@vizejs/native-win32-arm64-msvc";
        case "x64":
          return "@vizejs/native-win32-x64-msvc";
        default:
          throw new Error(`Unsupported architecture on Windows: ${arch}`);
      }
    default:
      throw new Error(`Unsupported OS: ${platform}`);
  }
}

function isPatinaBinding(binding: Partial<PatinaBinding>): binding is PatinaBinding {
  return (
    typeof binding.lintPatinaSfc === "function" && typeof binding.getPatinaRules === "function"
  );
}

export function loadBinding(): PatinaBinding {
  if (bindingCache) {
    return bindingCache;
  }

  const attemptedPackages = [getBindingPackageName(), FALLBACK_BINDING_PACKAGE];
  let lastError: unknown = null;

  for (const packageName of attemptedPackages) {
    try {
      const binding = require(packageName) as Partial<PatinaBinding>;
      if (!isPatinaBinding(binding)) {
        throw new Error(`${packageName} does not expose the Patina Oxlint bridge.`);
      }

      bindingCache = binding;
      return bindingCache;
    } catch (error) {
      lastError = error;
    }
  }

  const message = lastError instanceof Error && lastError.message ? ` ${lastError.message}` : "";
  throw new Error(
    `Failed to load the Vize native binding. Tried ${attemptedPackages.join(", ")}.${message}`,
  );
}
