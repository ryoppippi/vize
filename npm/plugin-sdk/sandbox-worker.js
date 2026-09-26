// This program executes only inside the isolated container. Its VM is not a
// security boundary: the container supplies filesystem/network/process limits.
import { readFileSync, writeSync } from "node:fs";
import { runInThisContext } from "node:vm";

const request = JSON.parse(readFileSync(0, "utf8"));
const sdk = await import(request.sdk);
const { definition, batchJson } = request;
const configuration = freeze(definition.configuration ?? null);
const callback = (source) => {
  // Evaluation happens only in the container; the VM supplies no isolation.
  const fn = runInThisContext(`"use strict"; (${source})`);
  if (typeof fn !== "function") throw new TypeError("callback source must be a function");
  return (batch) => fn(batch, configuration);
};
const common = {
  name: definition.name,
  version: definition.version,
  visit: definition.visit,
  demands: definition.demands,
  cacheInputs: definition.cacheInputs,
};
let plugin;
switch (definition.family) {
  case "rule":
    plugin = sdk.definePlugin({
      ...common,
      rules: Object.fromEntries(
        Object.entries(definition.rules).map(([name, source]) => [name, callback(source)]),
      ),
    });
    break;
  case "transform":
    plugin = sdk.defineTransformPlugin({ ...common, transform: callback(definition.callback) });
    break;
  case "provider":
    plugin = sdk.defineFactProvider({
      ...common,
      provides: definition.provides,
      provide: callback(definition.callback),
    });
    break;
  case "formatter":
  case "output":
    plugin = sdk.defineOutputPlugin({
      ...common,
      family: definition.family,
      output: callback(definition.callback),
    });
    break;
  default:
    throw new TypeError("unsupported sandbox family");
}
const output = plugin.run(batchJson);
// The parent bounds every stdout/stderr byte, including unsolicited writes.
writeSync(1, output);

function freeze(value) {
  if (value && typeof value === "object") {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}
