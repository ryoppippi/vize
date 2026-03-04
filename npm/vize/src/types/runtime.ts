import type { VizeConfig } from "./generated.js";

// ============================================================================
// TS-specific runtime types (cannot be expressed in Pkl)
// ============================================================================

export type MaybePromise<T> = T | Promise<T>;

export interface ConfigEnv {
  mode: string;
  command: "serve" | "build" | "check" | "lint" | "fmt";
  isSsrBuild?: boolean;
}

export type UserConfigExport = VizeConfig | ((env: ConfigEnv) => MaybePromise<VizeConfig>);

// ============================================================================
// LoadConfigOptions
// ============================================================================

export interface LoadConfigOptions {
  /**
   * Config file search mode
   * - 'root': Search only in the specified root directory
   * - 'auto': Search from cwd upward until finding a config file
   * - 'none': Don't load config file
   * @default 'root'
   */
  mode?: "root" | "auto" | "none";

  /**
   * Custom config file path (overrides automatic search)
   */
  configFile?: string;

  /**
   * Config environment for dynamic config resolution
   */
  env?: ConfigEnv;
}
