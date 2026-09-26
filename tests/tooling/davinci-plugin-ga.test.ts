import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";
import { applyFixes, definePlugin } from "../../npm/plugin-sdk/index.js";
import booleans from "./fixtures/davinci-plugin-sdk/boolean-attributes.mjs";
import team from "./fixtures/davinci-plugin-sdk/team-conventions.mjs";

const native = createRequire(import.meta.url)("../../npm/native/index.js");
const source = `<script setup>const items = [{ id: "a" }]</script>
<template>日本語😀<button v-for="(item, i) in items" :key="i" :disabled="true">Go</button><Child :disabled="true" /></template>`;
const options = { filename: "Buttons.vue", cache: true, validateDeterminism: true };

test("two team rules run cached, deterministic, attributed and host-confined autofixes", () => {
  const first = native.lintWithPlugins(source, [team, booleans], options);
  const hit = native.lintWithPlugins(source, [team, booleans], options);
  assert.deepEqual(
    first.diagnostics.map((row) => row.ruleId),
    ["team-conventions/no-index-key", "boolean-attributes/prefer-boolean-shorthand"],
  );
  assert.deepEqual(hit.diagnostics, first.diagnostics);
  assert.deepEqual(hit.fixes, first.fixes);
  assert.deepEqual(
    first.plugins.map((row) => row.cached),
    [false, false],
  );
  assert.deepEqual(
    hit.plugins.map((row) => [row.cached, row.nodes, row.batchBytes, row.jsNs]),
    [
      [true, 0, 0, 0],
      [true, 0, 0, 0],
    ],
  );
  for (const cost of first.plugins) {
    assert.ok(cost.elapsedNs >= cost.jsNs && cost.jsNs > 0);
    assert.equal(cost.reports, 1);
    assert.ok(cost.contentKey && cost.batchBytes > 0);
  }
  const [fix] = first.fixes;
  assert.deepEqual(fix, {
    ruleId: "boolean-attributes/prefer-boolean-shorthand",
    plugin: "boolean-attributes",
    start: Buffer.byteLength(source.slice(0, source.indexOf(":disabled"))),
    end: Buffer.byteLength(
      source.slice(0, source.indexOf(":disabled") + ':disabled="true"'.length),
    ),
    text: "disabled",
  });
  const fixed = applyFixes(source, first.fixes);
  assert.equal(fixed, source.replace(':disabled="true"', "disabled"));
  assert.deepEqual(native.lintWithPlugins(fixed, [booleans], options).diagnostics, []);
});

test("cached misses audit identical batches before writing a nondeterministic result", () => {
  let calls = 0;
  const plugin = definePlugin({
    name: "changing-output",
    version: "1",
    cacheInputs: [],
    rules: {
      check(ctx) {
        ctx.report(ctx.nodes[0], String(++calls));
      },
    },
  });
  assert.throws(
    () => native.lintWithPlugins(source, [plugin], { ...options, validateDeterminism: false }),
    /identical visit batches produced different diagnostics or fixes/,
  );
  assert.equal(calls, 2);
  assert.throws(
    () => native.lintWithPlugins(source, [plugin], options),
    /identical visit batches produced different diagnostics or fixes/,
  );
  assert.equal(calls, 4);
});

test("uncached determinism audits fixes as well as messages", () => {
  let calls = 0;
  const plugin = definePlugin({
    name: "changing-fix",
    version: "1",
    rules: {
      check(ctx) {
        ctx.report(ctx.nodes[0], "same", String(++calls));
      },
    },
  });
  assert.throws(
    () => native.lintWithPlugins(source, [plugin], { validateDeterminism: true }),
    /different diagnostics or fixes/,
  );
});
