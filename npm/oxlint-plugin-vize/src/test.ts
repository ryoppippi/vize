import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { extractSfcBlocks, formatBlockLabel, getDiagnosticBlock } from "./sfc-blocks.ts";

const packageDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(packageDir, "../../..");
const pluginEntry = path.join(workspaceRoot, "npm/oxlint-plugin-vize/dist/index.mjs");
const fixtureDir = path.join(workspaceRoot, "__agent_only", "oxlint-plugin-vize-test");
const configPath = path.join(fixtureDir, ".oxlintrc.json");
const noHelpConfigPath = path.join(fixtureDir, ".oxlintrc.no-help.json");
const shortHelpConfigPath = path.join(fixtureDir, ".oxlintrc.short-help.json");
const vuePath = path.join(fixtureDir, "App.vue");
const snapshotsDir = path.join(packageDir, "__snapshots__");
const ansiEscapePattern = new RegExp(String.raw`\u001B\[[0-9;]*m`, "gu");

function findOxlintBin() {
  const pnpmStoreDir = path.join(workspaceRoot, "node_modules", ".pnpm");
  const candidates = fs
    .readdirSync(pnpmStoreDir)
    .filter((entry) => entry.startsWith("oxlint@"))
    .sort((left, right) => right.localeCompare(left))
    .map((entry) => path.join(pnpmStoreDir, entry, "node_modules", "oxlint", "bin", "oxlint"))
    .filter((entry) => fs.existsSync(entry));

  const match = candidates[0];
  if (match == null) {
    throw new Error(`Unable to locate the oxlint binary in ${pnpmStoreDir}`);
  }

  return match;
}

const oxlintBin = findOxlintBin();

fs.rmSync(fixtureDir, { force: true, recursive: true });
fs.mkdirSync(fixtureDir, { recursive: true });

fs.writeFileSync(
  configPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      rules: {
        "no-unused-vars": "off",
        "vize/vue/require-v-for-key": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  noHelpConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          showHelp: false,
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/vue/require-v-for-key": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  shortHelpConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          helpLevel: "short",
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/vue/require-v-for-key": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  vuePath,
  `<script setup lang="ts">
const items = [1]
</script>
<template>
  <ul>
    <li v-for="item in items">{{ item }}</li>
  </ul>
</template>
`,
);

function runOxlint(args: readonly string[]) {
  let output = "";
  let exitCode = 0;

  try {
    execFileSync(oxlintBin, args, {
      cwd: fixtureDir,
      encoding: "utf8",
      stdio: "pipe",
    });
  } catch (error) {
    const execError = error as {
      status?: number;
      stdout?: string | Buffer;
      stderr?: string | Buffer;
    };
    exitCode = execError.status ?? 1;
    output = String(execError.stdout ?? "") + String(execError.stderr ?? "");
  }

  return {
    exitCode,
    output: normalizeOutput(output),
  };
}

function normalizeOutput(output: string): string {
  return output
    .replace(ansiEscapePattern, "")
    .replace(/^WARNING: JS plugins are experimental and not subject to semver\.\n/gmu, "")
    .replace(
      /^Breaking changes are possible while JS plugins support is under development\.\n/gmu,
      "",
    )
    .replace(/"start_time": [0-9.]+/gu, '"start_time": 0')
    .replace(/^Finished in .*$/gmu, "")
    .trim();
}

function readSnapshot(name: string): string {
  return fs.readFileSync(path.join(snapshotsDir, name), "utf8").trim();
}

const defaultRun = runOxlint(["-c", ".oxlintrc.json", "App.vue"]);
assert.notEqual(defaultRun.exitCode, 0, "oxlint should fail when Patina reports an error");
assert.match(
  defaultRun.output,
  /vize\(vue\/require-v-for-key\)/,
  "Vize rule name should be surfaced",
);
assert.match(
  defaultRun.output,
  /Elements in iteration expect to have 'v-bind:key' directives\. \(at <template>:6:9\)/u,
  "Template diagnostics should expose the real Vue location inline in the summary",
);
assert.doesNotMatch(
  defaultRun.output,
  /^  \| Source:/mu,
  "The inline source snippet should be gone",
);
assert.match(
  defaultRun.output,
  /^  \|     Help:/mu,
  "Default output should still include help text",
);

const stylishRun = runOxlint(["-c", ".oxlintrc.no-help.json", "-f", "stylish", "App.vue"]);
assert.notEqual(stylishRun.exitCode, 0, "stylish formatter should still report Patina failures");
assert.match(
  stylishRun.output,
  /App\.vue[\s\S]*2:1[\s\S]*Elements in iteration expect to have 'v-bind:key' directives\. \(at <template>:6:9\)[\s\S]*^    Details:$[\s\S]*^      Element: <li>  vize\(vue\/require-v-for-key\)$/mu,
  "Stylish formatter should keep the output concise while surfacing the real Vue location",
);
assert.doesNotMatch(stylishRun.output, /^Help:/mu, "showHelp: false should omit the help section");
assert.doesNotMatch(
  stylishRun.output,
  /^    Location:/mu,
  "Stylish formatter should avoid a separate location block",
);
assert.equal(stylishRun.output, readSnapshot("stylish-no-help-output.txt"));

const shortHelpRun = runOxlint(["-c", ".oxlintrc.short-help.json", "-f", "stylish", "App.vue"]);
assert.notEqual(shortHelpRun.exitCode, 0, "short help should still report Patina failures");
assert.match(
  shortHelpRun.output,
  /^    Help:\n      Why:/mu,
  "short help should keep only the first help line",
);
assert.equal(shortHelpRun.output, readSnapshot("stylish-short-help-output.txt"));

const jsonRun = runOxlint(["-c", ".oxlintrc.no-help.json", "-f", "json", "App.vue"]);
assert.notEqual(jsonRun.exitCode, 0, "json formatter should still report Patina failures");
assert.equal(jsonRun.output, readSnapshot("json-no-help-output.txt"));

const sampleBlocks = extractSfcBlocks(
  `<script setup lang="ts">\nconst count = 1\n</script>\n<template>\n  <div>{{ count }}</div>\n</template>\n<style scoped>\n.foo {}\n</style>\n<i18n>\n{}\n</i18n>\n`,
);
assert.deepEqual(
  sampleBlocks.map((block) => formatBlockLabel(block)),
  ["<script setup>", "<template>", "<style>", "<i18n>"],
  "SFC block extraction should classify common Vue block types",
);
assert.equal(
  formatBlockLabel(
    getDiagnosticBlock(
      {
        rule: "vize/vue/mock",
        severity: "error",
        message: "Mock error. Detail: extra context",
        location: {
          start: { line: 5, column: 3, offset: 0 },
          end: { line: 5, column: 8, offset: 0 },
        },
        help: null,
      },
      sampleBlocks,
    ),
  ),
  "<template>",
  "Diagnostics should map back to their containing SFC block",
);

console.log("✅ oxlint-plugin-vize integration tests passed!");
