// Native declarations are copied verbatim from the real generated package.
import {
  compileWithOutputPlugins,
  compileWithTransformPlugins,
  lintWithPlugins,
} from "@vizejs/native";
import {
  plugin,
  provider,
  transform,
  formatter,
  output,
  sandbox,
  sandboxProvider,
  sandboxTransform,
  sandboxFormatter,
  sandboxOutput,
} from "./packed-authoring.js";

lintWithPlugins("<template><button/></template>", [plugin, sandbox], {
  cache: true,
  factProviders: [provider, sandboxProvider],
});
compileWithTransformPlugins("<button/>", [transform, sandboxTransform], {
  filename: "Button.vue",
  cache: true,
});
compileWithOutputPlugins("<button/>", [formatter, output, sandboxFormatter, sandboxOutput], {
  mode: "module",
});
