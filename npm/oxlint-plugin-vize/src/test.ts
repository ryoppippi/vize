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
const happyPathStyleConfigPath = path.join(fixtureDir, ".oxlintrc.happy-path-style.json");
const essentialStyleConfigPath = path.join(fixtureDir, ".oxlintrc.essential-style.json");
const happyPathScriptConfigPath = path.join(fixtureDir, ".oxlintrc.happy-path-script.json");
const opinionatedScriptConfigPath = path.join(fixtureDir, ".oxlintrc.opinionated-script.json");
const vuePath = path.join(fixtureDir, "App.vue");
const scopedStyleVuePath = path.join(fixtureDir, "ScopedStyle.vue");
const optionsApiVuePath = path.join(fixtureDir, "OptionsApi.vue");
const snapshotsDir = path.join(packageDir, "__snapshots__");
const ansiEscapePattern = new RegExp(String.raw`\u001B\[[0-9;]*m`, "gu");
const workspaceRootPattern = new RegExp(escapeRegExp(workspaceRoot), "gu");

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
  happyPathStyleConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          helpLevel: "none",
          preset: "happy-path",
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/vue/require-scoped-style": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  essentialStyleConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          helpLevel: "none",
          preset: "essential",
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/vue/require-scoped-style": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  happyPathScriptConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          helpLevel: "none",
          preset: "happy-path",
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/script/no-options-api": "error",
      },
    },
    null,
    2,
  ),
);

fs.writeFileSync(
  opinionatedScriptConfigPath,
  JSON.stringify(
    {
      plugins: ["vue"],
      jsPlugins: [pluginEntry],
      settings: {
        vize: {
          helpLevel: "none",
          preset: "opinionated",
        },
      },
      rules: {
        "no-unused-vars": "off",
        "vize/script/no-options-api": "error",
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

fs.writeFileSync(
  scopedStyleVuePath,
  `<script setup lang="ts">
const count = 1
</script>
<template>
  <div class="foo">{{ count }}</div>
</template>
<style>
.foo { color: red; }
</style>
`,
);

fs.writeFileSync(
  optionsApiVuePath,
  `<script lang="ts">
export default {
  data() {
    return {
      count: 1,
    };
  },
};
</script>
<template>
  <div>{{ count }}</div>
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
    .replace(workspaceRootPattern, "<workspaceRoot>")
    .replace(/^WARNING: JS plugins are experimental and not subject to semver\.\n/gmu, "")
    .replace(
      /^Breaking changes are possible while JS plugins support is under development\.\n/gmu,
      "",
    )
    .replace(/^\s*\[tsgo\] Using: .*$/gmu, "")
    .replace(/"start_time": [0-9.]+/gu, '"start_time": 0')
    .replace(/^Finished in .*$/gmu, "")
    .trim();
}

function escapeRegExp(value: string): string {
  return value.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
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

const happyPathStyleRun = runOxlint([
  "-c",
  ".oxlintrc.happy-path-style.json",
  "-f",
  "stylish",
  "ScopedStyle.vue",
]);
assert.notEqual(
  happyPathStyleRun.exitCode,
  0,
  "happy-path should report require-scoped-style when the rule is enabled",
);
assert.match(
  happyPathStyleRun.output,
  /vize\(vue\/require-scoped-style\)/,
  "happy-path should keep reporting rules that belong to the default preset",
);
assert.equal(
  happyPathStyleRun.output,
  readSnapshot("stylish-happy-path-require-scoped-style-output.txt"),
);

const essentialStyleRun = runOxlint([
  "-c",
  ".oxlintrc.essential-style.json",
  "-f",
  "stylish",
  "ScopedStyle.vue",
]);
assert.equal(
  essentialStyleRun.exitCode,
  0,
  "essential preset should skip require-scoped-style even when the Oxlint rule is configured",
);
assert.doesNotMatch(
  essentialStyleRun.output,
  /vize\(vue\/require-scoped-style\)/,
  "essential preset should not surface happy-path-only rules",
);

const happyPathScriptRun = runOxlint([
  "-c",
  ".oxlintrc.happy-path-script.json",
  "-f",
  "stylish",
  "OptionsApi.vue",
]);
assert.equal(
  happyPathScriptRun.exitCode,
  0,
  "happy-path should not enable opinionated script rules by default",
);
assert.doesNotMatch(
  happyPathScriptRun.output,
  /vize\(script\/no-options-api\)/,
  "happy-path should keep script/no-options-api opt-in",
);

const opinionatedScriptRun = runOxlint([
  "-c",
  ".oxlintrc.opinionated-script.json",
  "-f",
  "stylish",
  "OptionsApi.vue",
]);
assert.notEqual(
  opinionatedScriptRun.exitCode,
  0,
  "opinionated preset should enable script/no-options-api",
);
assert.match(
  opinionatedScriptRun.output,
  /vize\(script\/no-options-api\)/,
  "opinionated preset should surface opinionated script diagnostics",
);
assert.equal(
  opinionatedScriptRun.output,
  readSnapshot("stylish-opinionated-no-options-api-output.txt"),
);

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
