export type {
  RuleSeverity,
  RuleCategory,
  VizeConfig,
  CompilerConfig,
  VitePluginConfig,
  LinterConfig,
  TypeCheckerConfig,
  FormatterConfig,
  LanguageServerConfig,
  MuseaVrtConfig,
  MuseaA11yConfig,
  MuseaAutogenConfig,
  MuseaConfig,
  MuseaViewport,
  GlobalTypeDeclaration,
  GlobalTypesConfig,
} from "./generated.js";

/**
 * @deprecated Use `LanguageServerConfig`.
 */
export type LspConfig = import("./generated.js").LanguageServerConfig;

export type { MaybePromise, ConfigEnv, UserConfigExport, LoadConfigOptions } from "./runtime.js";
