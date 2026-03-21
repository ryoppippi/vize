import fs from "node:fs";
import path from "node:path";

import { hasScriptLikeBlock, appendScriptlessWorkaround } from "../workaround.js";

export interface PreparedWorkaroundFiles {
  appendedArgs: string[];
  cleanup(): void;
  pathReplacements: ReadonlyMap<string, string>;
  usedScriptlessWorkaround: boolean;
}

export function prepareScriptlessWorkaroundFiles(
  cwd: string,
  filenames: readonly string[],
): PreparedWorkaroundFiles {
  const tempDir = path.join(cwd, "__oxlint_plugin_vize_temp__", String(process.pid));
  const ignoreArgs: string[] = [];
  const tempArgs: string[] = [];
  const pathReplacements = new Map<string, string>();
  let counter = 0;

  for (const filename of filenames) {
    const source = fs.readFileSync(filename, "utf8");
    if (hasScriptLikeBlock(source)) {
      continue;
    }

    const relativeFilename = path.relative(cwd, filename);
    const tempFilename = path.join(tempDir, `${counter}-${path.basename(filename)}`);
    counter += 1;

    fs.mkdirSync(path.dirname(tempFilename), { recursive: true });
    fs.writeFileSync(tempFilename, appendScriptlessWorkaround(source, filename));

    ignoreArgs.push("--ignore-pattern", toCliPath(relativeFilename));
    tempArgs.push(tempFilename);
    pathReplacements.set(tempFilename, filename);
  }

  return {
    appendedArgs: [...ignoreArgs, ...tempArgs],
    cleanup() {
      if (pathReplacements.size === 0) {
        return;
      }

      fs.rmSync(tempDir, { force: true, recursive: true });
    },
    pathReplacements,
    usedScriptlessWorkaround: pathReplacements.size > 0,
  };
}

function toCliPath(filename: string): string {
  return filename.split(path.sep).join("/");
}
