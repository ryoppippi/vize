import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const native = createRequire(import.meta.url)("../../npm/native/index.js");
const groups = [
  "component-signature",
  "prop-types",
  "emit-types",
  "slot-types",
  "reactivity-classes",
  "component-references",
];
const source = `<script setup lang="ts" generic="T extends 'a b'">
import ChildAlias from './Child.vue'
import type { Public as Alias } from './types'
import { ref } from 'vue'
defineOptions({ name: 'PublicComponent' })
const props = defineProps<{ label: 'a b'; choice: Alias; value?: T }>()
defineEmits<{ save: [value: T] }>()
defineSlots<{ default(props: { item: T }): any }>()
const model = defineModel<string>('title')
const count = ref(0)
defineExpose({ count })
function privateBody() { return 1 }
</script><template><ChildAlias>{{ props.label }} {{ privateBody() }}</ChildAlias></template>`;

function inspect(input: string, demands = groups) {
  let facts;
  const plugin = {
    name: "production-alpha-inspection",
    version: "1",
    fingerprint: "inspect-alpha-v1",
    cacheInputs: [],
    visit: [],
    demands,
    run(json: string) {
      const batch = JSON.parse(json);
      if (facts !== undefined) assert.deepEqual(batch.facts, facts);
      facts = batch.facts;
      return "[]";
    },
  };
  const output = native.lintWithPlugins(input, [plugin], {
    filename: "Public.vue",
    validateDeterminism: true,
  });
  assert.deepEqual(output.diagnostics, []);
  return facts as Record<string, [string, Record<string, any>][]>;
}

const row = (facts: ReturnType<typeof inspect>, group: string, name: string) => {
  const found = facts[group].find(([key]) => key === name);
  assert.ok(found, `${group} does not contain ${name}`);
  return found[1];
};

test("six production alpha groups cross the native batch with exact typed fields", () => {
  const facts = inspect(source);
  assert.deepEqual(Object.keys(facts).sort(), [...groups].sort());
  for (const group of groups) {
    assert.ok(facts[group].length > 0);
    for (const [key, contract] of facts[group]) {
      assert.equal(typeof key, "string");
      assert.equal(contract.schema, 1);
    }
  }
  const signature = row(facts, "component-signature", "Public.vue");
  assert.equal(signature.generic, "T extends 'a b'");
  assert.equal(signature.name, "Public.vue");
  assert.equal(signature.declared_name, "PublicComponent");
  assert.equal(signature.script_setup, true);
  const label = row(facts, "prop-types", "label");
  assert.equal(label.type, "'a b'");
  assert.equal(label.default, null);
  assert.equal(label.model_modifiers, null);
  const dependency = row(facts, "prop-types", "choice").type_dependencies;
  assert.equal(dependency.complete, false);
  const imported = dependency.declarations.find((entry: any) => entry.name === "Alias");
  assert.equal(imported.module, "./types");
  assert.equal(imported.export, "Public");
  assert.equal(imported.body, null);
  assert.ok(
    facts["component-references"].some(
      ([, value]) => value.module === "./Child.vue" && value.export === "default",
    ),
  );
  assert.ok(facts["reactivity-classes"].some(([, value]) => value.name === "count"));
});

test("alpha demands expose only their declared pages while beta demands compose", () => {
  const props = inspect(source, ["prop-types"]);
  assert.deepEqual(Object.keys(props), ["prop-types"]);
  const both = inspect(source, ["prop-types", "bindings"]);
  assert.deepEqual(Object.keys(both).sort(), ["bindings", "prop-types"]);
  assert.deepEqual(both["prop-types"], props["prop-types"]);
  assert.ok(both.bindings.some(([key]) => key === "count"));
});

test("body edits preserve alpha values and authored literal edits invalidate interfaces", () => {
  const before = inspect(source);
  const body = source
    .replace("return 1", "return 2")
    .replace("props.label }}", "props.label }} changed body");
  assert.deepEqual(inspect(body), before);
  const changed = inspect(source.replace("label: 'a b'", "label: 'ab'"));
  assert.notDeepEqual(changed["prop-types"], before["prop-types"]);
  assert.equal(row(changed, "prop-types", "label").type, "'ab'");
  assert.deepEqual(changed["component-references"], before["component-references"]);
});

test("alpha fact requests reject unsupported foreign scripts", () => {
  assert.throws(
    () =>
      inspect('<script setup lang="moonbit">let x=1</script><template>{{x}}</template>', [
        "prop-types",
      ]),
    /production JS plugin facts require a JavaScript or TypeScript script/,
  );
});
