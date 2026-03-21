import { spawnSync } from "node:child_process";

import { getLintTargets } from "./cli/args.js";
import { collectVueFilesFromTargets } from "./cli/files.js";
import { resolveOxlintCliEntrypoint } from "./cli/oxlint.js";
import { rewriteReportedPaths } from "./cli/output.js";
import { prepareScriptlessWorkaroundFiles } from "./cli/workaround-files.js";

function main(): void {
  const cwd = process.cwd();
  const forwardedArgs = process.argv.slice(2);
  const targets = getLintTargets(forwardedArgs);
  const vueFiles = collectVueFilesFromTargets(cwd, targets);
  const prepared = prepareScriptlessWorkaroundFiles(cwd, vueFiles);
  const oxlintEntrypoint = resolveOxlintCliEntrypoint(cwd);
  const args = [oxlintEntrypoint, ...forwardedArgs, ...prepared.appendedArgs];

  const result = spawnSync(process.execPath, args, {
    cwd,
    encoding: "utf8",
    stdio: "pipe",
  });

  prepared.cleanup();

  const stdout = rewriteReportedPaths(result.stdout ?? "", prepared.pathReplacements);
  const stderr = rewriteReportedPaths(result.stderr ?? "", prepared.pathReplacements);

  if (stdout) {
    process.stdout.write(stdout);
  }

  if (stderr) {
    process.stderr.write(stderr);
  }

  if (prepared.usedScriptlessWorkaround && forwardedArgs.includes("--fix")) {
    process.stderr.write(
      "\n[oxlint-plugin-vize] Scriptless SFC workaround is active; fixes are not applied back to original .vue files yet.\n",
    );
  }

  if (result.error) {
    throw result.error;
  }

  process.exit(result.status ?? 1);
}

main();
