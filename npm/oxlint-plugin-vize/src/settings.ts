import type { Context } from "@oxlint/plugins";

import type { HelpLevel, PatinaSettings } from "./model.js";

const HELP_LEVELS = new Set<HelpLevel>(["none", "short", "full"]);

export function isVueLikeFile(filename: string): boolean {
  return filename.endsWith(".vue");
}

export function getVizeSettings(context: Context): PatinaSettings {
  const settings = context.settings as Record<string, unknown>;
  return parseVizeSettings(settings.vize ?? settings.patina);
}

export function parseVizeSettings(vize: unknown): PatinaSettings {
  if (typeof vize !== "object" || vize === null || Array.isArray(vize)) {
    return {};
  }

  const vizeRecord = vize as Record<string, unknown>;
  const locale = vizeRecord.locale;
  const helpLevel = vizeRecord.helpLevel;
  const showHelp = vizeRecord.showHelp;
  const resolved: PatinaSettings = {};

  if (typeof locale === "string") {
    resolved.locale = locale;
  }

  if (typeof helpLevel === "string" && HELP_LEVELS.has(helpLevel as HelpLevel)) {
    resolved.helpLevel = helpLevel as HelpLevel;
    return resolved;
  }

  if (typeof showHelp === "boolean") {
    resolved.helpLevel = showHelp ? "full" : "none";
  }

  return resolved;
}

export function getCacheKey(filename: string, settings: PatinaSettings): string {
  return `${filename}::${settings.locale ?? ""}::${settings.helpLevel ?? ""}`;
}

if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  describe("parseVizeSettings", () => {
    it("reads locale and help level", () => {
      expect(parseVizeSettings({ locale: "ja", helpLevel: "short" })).toEqual({
        locale: "ja",
        helpLevel: "short",
      });
    });

    it("falls back to showHelp for compatibility", () => {
      expect(parseVizeSettings({ locale: "ja", showHelp: false })).toEqual({
        locale: "ja",
        helpLevel: "none",
      });
    });

    it("ignores invalid values", () => {
      expect(parseVizeSettings({ locale: 1, showHelp: "no", helpLevel: "verbose" })).toEqual({});
    });

    it("ignores non-object vize settings", () => {
      expect(parseVizeSettings(null)).toEqual({});
      expect(parseVizeSettings("ja")).toEqual({});
    });
  });
}
