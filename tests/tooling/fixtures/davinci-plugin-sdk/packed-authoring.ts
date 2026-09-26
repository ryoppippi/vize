import {
  BATCH_SCHEMA,
  applyFixes,
  defineFactProvider,
  defineOutputPlugin,
  definePlugin,
  defineTransformPlugin,
  runBatch,
  runRules,
  type VisitBatch,
} from "@vizejs/plugin-sdk";
import { applyFixes as subpathFixes } from "@vizejs/plugin-sdk/fixes";
import { createSandboxRunner, SANDBOX_IMAGE } from "@vizejs/plugin-sdk/sandbox";

export const plugin = definePlugin({
  name: "packed-types",
  version: "1",
  visit: ["ui.element"] as const,
  demands: ["packed/colors"] as const,
  cacheInputs: [] as const,
  rules: {
    contrast(ctx) {
      for (const node of ctx.nodes)
        ctx.report(node, ctx.facts<string>("packed/colors").get(node.id) ?? "none");
    },
  },
});
const batch: VisitBatch = {
  schema: BATCH_SCHEMA,
  parents: [-1],
  nodes: [{ id: 0, kind: "ui.element" }],
  facts: { "packed/colors": [[0, "red"]] },
};
runBatch(plugin, batch);
runRules(plugin, { nodes: batch.nodes, byId: new Map(), facts: () => new Map(), parent: () => -1 });
applyFixes("blue", [{ start: 0, end: 4, text: "red" }] as const);
subpathFixes("blue", [] as const);

export const provider = defineFactProvider({
  name: "packed",
  version: "1",
  provides: ["packed/colors"] as const,
  cacheInputs: [] as const,
  provide(input) {
    return { "packed/colors": input.nodes.map((node) => [node.id, "red"] as const) };
  },
});
export const transform = defineTransformPlugin({
  name: "packed-transform",
  version: "1",
  cacheInputs: [] as const,
  transform(input) {
    return input.nodes.map((node) => ({
      kind: "replace-static-attribute" as const,
      node: node.id,
      name: "disabled",
      value: null,
    }));
  },
});
export const formatter = defineOutputPlugin({
  name: "packed-formatter",
  version: "1",
  cacheInputs: [] as const,
  family: "formatter",
  output: () => [{ start: 0, end: 0, text: "\n" }] as const,
});
export const output = defineOutputPlugin({
  name: "packed-output",
  version: "1",
  cacheInputs: [] as const,
  family: "output",
  output: () => [{ placement: "append", comment: "packed" }] as const,
});
export const sandbox = createSandboxRunner({
  name: "packed-sandbox",
  version: "1",
  cacheInputs: [] as const,
  rules: { check: "(ctx) => {}" },
});
export const sandboxProvider = createSandboxRunner({
  name: "packed",
  version: "1",
  family: "provider",
  provides: ["packed/colors"] as const,
  cacheInputs: [] as const,
  callback: "(batch) => ({ 'packed/colors': [] })",
});
export const sandboxTransform = createSandboxRunner({
  name: "packed-transform",
  version: "1",
  family: "transform",
  cacheInputs: [] as const,
  callback: "(batch) => []",
});
export const sandboxFormatter = createSandboxRunner({
  name: "packed-formatter",
  version: "1",
  family: "formatter",
  cacheInputs: [] as const,
  callback: "(batch) => []",
});
export const sandboxOutput = createSandboxRunner({
  name: "packed-output",
  version: "1",
  family: "output",
  cacheInputs: [] as const,
  callback: "(batch) => []",
});
SANDBOX_IMAGE satisfies string;
