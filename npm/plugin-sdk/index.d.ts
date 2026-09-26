import type { FactGroups } from "./facts.js";

export declare const BATCH_SCHEMA: 1;
export interface PluginNode {
  id: number;
  parent?: number;
  kind: string;
  name?: string;
  value?: string;
  attrs?: Array<[string, string | null]>;
  alias?: { value: string; key: string | null; index: string | null };
}
export interface ScopeEntry {
  name: string;
  position: "value" | "key" | "index" | "slot";
}
export interface RuleContext {
  readonly nodes: readonly PluginNode[];
  facts(name: "templateScopes"): ReadonlyMap<number, readonly ScopeEntry[]>;
  facts<K extends keyof FactGroups>(name: K): FactGroups[K];
  facts<T = unknown>(name: string): ReadonlyMap<string | number, T>;
  ancestors(node: PluginNode, kind: string): IterableIterator<PluginNode>;
  /** Replace the whole reported node span; ranges are assigned by the host. */
  report(node: PluginNode, message: string, fix?: string): void;
}
export interface PluginDefinition {
  name: string;
  version: string;
  visit?: readonly string[];
  demands?: readonly string[];
  /** Explicit list of all captured configuration and ambient inputs. */
  cacheInputs?: readonly { name: string; value: string }[];
  rules: Readonly<Record<string, (context: RuleContext) => void>>;
}
/** Frozen at runtime. Array declarations interoperate with generated native descriptors. */
export interface Plugin extends Omit<PluginDefinition, "visit" | "demands" | "cacheInputs"> {
  readonly visit?: string[];
  readonly demands: string[];
  readonly cacheInputs?: { name: string; value: string }[];
  readonly fingerprint: string;
  run(batchJson: string): string;
}
export interface VisitBatch {
  schema: 1;
  plugin?: string;
  file?: string;
  parents: number[];
  nodes: PluginNode[];
  facts: Record<string, Array<[string | number, unknown]>>;
}
export declare function definePlugin(definition: PluginDefinition): Plugin;
export declare function runBatch(
  plugin: Plugin,
  batch: VisitBatch,
): Array<{ rule: string; node: number; message: string; fix?: string }>;

export declare function runRules(
  plugin: Plugin,
  source: {
    nodes: readonly PluginNode[];
    byId: ReadonlyMap<number, PluginNode>;
    facts(name: string): ReadonlyMap<string | number, unknown>;
    parent(id: number): number;
  },
): Array<{ rule: string; node: number; message: string; fix?: string }>;

export { defineFactProvider, defineOutputPlugin, defineTransformPlugin } from "./stages.js";
export { applyFixes } from "./fixes.js";

export type * from "./facts.js";
