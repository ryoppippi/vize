import type { Context } from "@oxlint/plugins";

import type { PatinaSettings } from "./model.js";

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
  const showHelp = vizeRecord.showHelp;
  const resolved: PatinaSettings = {};

  if (typeof locale === "string") {
    resolved.locale = locale;
  }

  if (typeof showHelp === "boolean") {
    resolved.showHelp = showHelp;
  }

  return resolved;
}

export function getCacheKey(filename: string, settings: PatinaSettings): string {
  return `${filename}::${settings.locale ?? ""}`;
}

if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  describe("parseVizeSettings", () => {
    it("reads locale and showHelp flags", () => {
      expect(parseVizeSettings({ locale: "ja", showHelp: false })).toEqual({
        locale: "ja",
        showHelp: false,
      });
    });

    it("ignores invalid values", () => {
      expect(parseVizeSettings({ locale: 1, showHelp: "no" })).toEqual({});
    });

    it("ignores non-object vize settings", () => {
      expect(parseVizeSettings(null)).toEqual({});
      expect(parseVizeSettings("ja")).toEqual({});
    });
  });
}
