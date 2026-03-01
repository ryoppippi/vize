/**
 * Individual route handler functions for the Musea gallery API.
 *
 * Extracted from api-routes.ts to keep file sizes manageable.
 * These handle GET /api/arts/:path/... sub-routes.
 */

import fs from "node:fs";
import path from "node:path";

import type { ApiRoutesContext, SendJson, SendError } from "./index.js";
import { loadNative, analyzeSfcFallback } from "../native-loader.js";

/** GET /api/arts/:path/source */
export async function handleArtSource(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    sendJson({ source, path: artPath });
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/palette */
export async function handleArtPalette(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    const binding = loadNative();
    let palette: {
      title: string;
      controls: Array<{
        name: string;
        control: string;
        default_value?: unknown;
        description?: string;
        required: boolean;
        options: Array<{ label: string; value: unknown }>;
        range?: { min: number; max: number; step?: number };
        group?: string;
      }>;
      groups: string[];
      json: string;
      typescript: string;
    };
    if (binding.generateArtPalette) {
      palette = binding.generateArtPalette(source, {
        filename: artPath,
      });
    } else {
      palette = {
        title: art.metadata.title,
        controls: [],
        groups: [],
        json: "{}",
        typescript: "",
      };
    }

    // If the native palette returned no controls, try JS-based SFC analysis
    if (palette.controls.length === 0 && art.metadata.component) {
      const resolvedComponentPath = path.isAbsolute(art.metadata.component)
        ? art.metadata.component
        : path.resolve(path.dirname(artPath), art.metadata.component);
      try {
        const componentSource = await fs.promises.readFile(resolvedComponentPath, "utf-8");
        const analysis = binding.analyzeSfc
          ? binding.analyzeSfc(componentSource, {
              filename: resolvedComponentPath,
            })
          : analyzeSfcFallback(componentSource, {
              filename: resolvedComponentPath,
            });

        if (analysis.props.length > 0) {
          palette.controls = analysis.props.map((prop) => {
            let control = "text";
            if (prop.type === "boolean") control = "boolean";
            else if (prop.type === "number") control = "number";
            else if (prop.type.includes("|") && !prop.type.includes("=>")) {
              control = "select";
            }

            const options: Array<{ label: string; value: unknown }> = [];
            if (control === "select") {
              const optionMatches = prop.type.match(/"([^"]+)"/g);
              if (optionMatches) {
                for (const opt of optionMatches) {
                  const val = opt.replace(/"/g, "");
                  options.push({ label: val, value: val });
                }
              }
            }

            return {
              name: prop.name,
              control,
              default_value:
                prop.default_value !== undefined
                  ? prop.default_value === "true"
                    ? true
                    : prop.default_value === "false"
                      ? false
                      : typeof prop.default_value === "string" && prop.default_value.startsWith('"')
                        ? prop.default_value.replace(/^"|"$/g, "")
                        : prop.default_value
                  : undefined,
              description: undefined,
              required: prop.required,
              options,
              range: undefined,
              group: undefined,
            };
          });

          palette.json = JSON.stringify(
            { title: palette.title, controls: palette.controls },
            null,
            2,
          );
          palette.typescript = `export interface ${palette.title}Props {\n${palette.controls
            .map(
              (c) =>
                `  ${c.name}${c.required ? "" : "?"}: ${
                  c.control === "boolean"
                    ? "boolean"
                    : c.control === "number"
                      ? "number"
                      : c.control === "select"
                        ? c.options.map((o) => `"${String(o.value)}"`).join(" | ")
                        : "string"
                };`,
            )
            .join("\n")}\n}\n`;
        }
      } catch {
        // Ignore errors reading component file
      }
    }

    sendJson(palette);
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/analysis */
export async function handleArtAnalysis(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const resolvedComponentPath =
      art.isInline && art.componentPath
        ? art.componentPath
        : art.metadata.component
          ? path.isAbsolute(art.metadata.component)
            ? art.metadata.component
            : path.resolve(path.dirname(artPath), art.metadata.component)
          : null;

    if (resolvedComponentPath) {
      const source = await fs.promises.readFile(resolvedComponentPath, "utf-8");
      const binding = loadNative();
      if (binding.analyzeSfc) {
        const analysis = binding.analyzeSfc(source, {
          filename: resolvedComponentPath,
        });
        sendJson(analysis);
      } else {
        const analysis = analyzeSfcFallback(source, {
          filename: resolvedComponentPath,
        });
        sendJson(analysis);
      }
    } else {
      sendJson({ props: [], emits: [] });
    }
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/docs */
export async function handleArtDocs(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): Promise<void> {
  const artPath = decodeURIComponent(match[1]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  try {
    const source = await fs.promises.readFile(artPath, "utf-8");
    const binding = loadNative();
    if (binding.generateArtDoc) {
      const doc = binding.generateArtDoc(source, {
        filename: artPath,
      });
      // Replace Self with component name and format indentation
      let markdown = doc.markdown || "";
      const componentName = art.metadata.title || "Component";
      markdown = markdown
        .replace(/<Self(\s|>|\/)/g, `<${componentName}$1`)
        .replace(/<\/Self>/g, `</${componentName}>`);
      // Fix indentation in code blocks
      markdown = markdown.replace(
        /```(\w*)\n([\s\S]*?)```/g,
        (_match: string, lang: string, code: string) => {
          const lines = code.split("\n");
          let minIndent = Infinity;
          for (const line of lines) {
            if (line.trim()) {
              const indent = line.match(/^(\s*)/)?.[1].length || 0;
              minIndent = Math.min(minIndent, indent);
            }
          }
          if (minIndent === Infinity) minIndent = 0;
          let formatted: string;
          if (minIndent > 0) {
            formatted = lines.map((line: string) => line.slice(minIndent)).join("\n");
          } else {
            let restIndent = Infinity;
            for (let i = 1; i < lines.length; i++) {
              if (lines[i].trim()) {
                const indent = lines[i].match(/^(\s*)/)?.[1].length || 0;
                restIndent = Math.min(restIndent, indent);
              }
            }
            if (restIndent === Infinity || restIndent === 0) {
              formatted = lines.join("\n");
            } else {
              formatted = lines
                .map((line: string, i: number) => (i === 0 ? line : line.slice(restIndent)))
                .join("\n");
            }
          }
          return "```" + lang + "\n" + formatted + "```";
        },
      );
      sendJson({ ...doc, markdown });
    } else {
      sendJson({
        markdown: "",
        title: art.metadata.title,
        variant_count: art.variants.length,
      });
    }
  } catch (e) {
    sendError(e instanceof Error ? e.message : String(e));
  }
}

/** GET /api/arts/:path/variants/:name/a11y */
export function handleArtA11y(
  ctx: ApiRoutesContext,
  match: RegExpMatchArray,
  sendJson: SendJson,
  sendError: SendError,
): void {
  const artPath = decodeURIComponent(match[1]);
  const _variantName = decodeURIComponent(match[2]);
  const art = ctx.artFiles.get(artPath);
  if (!art) {
    sendError("Art not found", 404);
    return;
  }

  // Return empty a11y results (populated after VRT --a11y run)
  sendJson({ violations: [], passes: 0, incomplete: 0 });
}
