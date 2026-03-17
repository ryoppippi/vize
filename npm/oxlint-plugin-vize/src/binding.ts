import type { PatinaLintResult, PatinaRuleMeta, PatinaSettings } from "./model.js";
import { loadBinding } from "./native.js";

export function lintPatina(
  source: string,
  filename: string,
  settings: PatinaSettings,
  enabledRules?: readonly string[],
): PatinaLintResult {
  return loadBinding().lintPatinaSfc(source, {
    filename,
    locale: settings.locale,
    help_level: settings.helpLevel,
    enabled_rules: enabledRules ? [...enabledRules] : undefined,
  });
}

export function getPatinaRules(): PatinaRuleMeta[] {
  return loadBinding().getPatinaRules();
}
