export type JsonValue =
  | null
  | boolean
  | number
  | string
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };
export interface HookIdentity {
  name: string;
  version: string;
  cacheInputs?: readonly { name: string; value: string }[];
}
export interface NativeHook extends HookIdentity {
  readonly fingerprint: string;
  run(batchJson: string): string;
}
export interface StaticAttributeTransformBatch {
  schema: 1;
  stage: "s2-precanonical-static-attributes";
  plugin: string;
  file: string;
  nodes: readonly {
    id: number;
    kind: "ui.element";
    tag: string;
    namespace: string;
    attrs: readonly { name: string; value: string | null }[];
  }[];
}
export interface StaticAttributeEdit {
  kind: "replace-static-attribute";
  node: number;
  name: string;
  value: string | null;
}
export declare function defineTransformPlugin(
  definition: HookIdentity & {
    transform(batch: StaticAttributeTransformBatch): readonly StaticAttributeEdit[];
  },
): NativeHook;
export interface CompiledArtifact {
  code: string;
  preamble: string;
  ast: unknown;
  map?: unknown;
  helpers: readonly string[];
  templates?: readonly string[] | null;
}
export interface OutputBatch {
  schema: 1;
  plugin: string;
  family: "formatter" | "output";
  offsetEncoding: "utf8";
  compiled: CompiledArtifact;
}
export interface FormatEdit {
  start: number;
  end: number;
  text: string;
}
export interface OutputAddition {
  placement: "prepend" | "append";
  comment: string;
}
export declare function defineOutputPlugin(
  definition: HookIdentity & {
    family: "formatter" | "output";
    output(batch: OutputBatch): readonly FormatEdit[] | readonly OutputAddition[];
  },
): NativeHook & { readonly family: "formatter" | "output" };
export interface FactProviderBatch {
  schema: 1;
  plugin: string;
  file: string;
  parents: readonly number[];
  nodes: readonly import("./index.js").PluginNode[];
  facts: Readonly<Record<string, readonly (readonly [string | number, JsonValue])[]>>;
}
export declare function defineFactProvider(
  definition: HookIdentity & {
    visit?: readonly string[];
    demands?: readonly string[];
    provides: readonly string[];
    provide(
      batch: FactProviderBatch,
    ): Readonly<Record<string, readonly (readonly [string | number, JsonValue])[]>>;
  },
): NativeHook & { readonly provides: readonly string[] };
