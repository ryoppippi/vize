import type { NativeHook, JsonValue, HookIdentity } from "./stages.js";

/** A standalone function expression. It receives (contextOrBatch, configuration).
 * Closures and host imports are unavailable; process access stays in the container.
 */
export type CallbackSource = string;
export interface SandboxIdentity extends HookIdentity {
  visit?: readonly string[];
  demands?: readonly string[];
  configuration?: JsonValue;
}
export type SandboxDefinition = SandboxIdentity &
  (
    | { family?: "rule"; rules: Readonly<Record<string, CallbackSource>> }
    | { family: "provider"; provides: readonly string[]; callback: CallbackSource }
    | { family: "transform" | "formatter" | "output"; callback: CallbackSource }
  );
export interface SandboxLimits {
  /** 100..60000 ms; default 5000. Cleanup adds at most 5000 ms. */
  timeoutMs?: number;
  /** 1024..4194304 bytes for serialized request and each output stream; default 1 MiB. */
  maxBytes?: number;
}
/** Requires Docker and a pre-pulled SANDBOX_IMAGE. There are no host mounts or
 * host environment variables, network is disabled, root is read-only, and each
 * invocation has fixed memory/CPU/PID limits and confirmed container cleanup.
 * Runtime filesystem contents remain readable. Native callbacks are trusted.
 */
export declare function createSandboxRunner(
  definition: SandboxDefinition,
  limits?: SandboxLimits,
): NativeHook;
export declare const SANDBOX_IMAGE: string;
