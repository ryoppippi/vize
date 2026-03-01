/**
 * Musea gallery API route handlers.
 *
 * Extracted from the main plugin to keep file sizes manageable.
 * Provides REST API endpoints consumed by the gallery UI:
 * - GET/POST/PUT/DELETE /api/tokens  (delegated to api-tokens.ts)
 * - GET /api/arts, /api/arts/:path, /api/arts/:path/source, etc.
 * - POST /api/preview-with-props
 * - POST /api/generate
 * - POST /api/run-vrt
 */

import type { IncomingMessage, ServerResponse } from "node:http";
import type { ResolvedConfig } from "vite";
import fs from "node:fs";
import path from "node:path";

import type { ArtFileInfo } from "../types/index.js";
import { generatePreviewModuleWithProps } from "../preview/index.js";
import { toPascalCase } from "../utils.js";
import {
  handleTokensUsage,
  handleTokensGet,
  handleTokensCreate,
  handleTokensUpdate,
  handleTokensDelete,
} from "../api-tokens.js";
import {
  handleArtSource,
  handleArtPalette,
  handleArtAnalysis,
  handleArtDocs,
  handleArtA11y,
} from "./handlers.js";

/** Dependencies injected from the plugin closure. */
export interface ApiRoutesContext {
  config: ResolvedConfig;
  artFiles: Map<string, ArtFileInfo>;
  tokensPath: string | undefined;
  basePath: string;
  resolvedPreviewCss: string[];
  resolvedPreviewSetup: string | null;
  processArtFile: (filePath: string) => Promise<void>;
  /** Reference to the dev server for VRT port resolution */
  getDevServerPort: () => number;
}

export type SendJson = (data: unknown, status?: number) => void;
export type SendError = (message: string, status?: number) => void;
export type ReadBody = () => Promise<string>;

type NextFn = () => void;

/** Helper to read the full request body as a string. */
function collectBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => resolve(body));
  });
}

/**
 * Create the API middleware handler for the Musea gallery.
 *
 * Returns a Connect-compatible middleware function that handles all
 * `/api/...` sub-routes under the configured basePath.
 */
export function createApiMiddleware(ctx: ApiRoutesContext) {
  return async (req: IncomingMessage, res: ServerResponse, next: NextFn) => {
    const sendJson: SendJson = (data: unknown, status = 200) => {
      res.statusCode = status;
      res.setHeader("Content-Type", "application/json");
      res.end(JSON.stringify(data));
    };

    const sendError: SendError = (message: string, status = 500) => {
      sendJson({ error: message }, status);
    };

    const readBody: ReadBody = () => collectBody(req);

    const url = req.url || "/";

    // --- GET /api/arts ---
    if (url === "/arts" && req.method === "GET") {
      sendJson(Array.from(ctx.artFiles.values()));
      return;
    }

    // --- Token routes (delegated to api-tokens.ts) ---
    if (url === "/tokens/usage" && req.method === "GET") {
      await handleTokensUsage(ctx, sendJson);
      return;
    }
    if (url === "/tokens" && req.method === "GET") {
      await handleTokensGet(ctx, sendJson);
      return;
    }
    if (url === "/tokens" && req.method === "POST") {
      await handleTokensCreate(ctx, readBody, sendJson, sendError);
      return;
    }
    if (url === "/tokens" && req.method === "PUT") {
      await handleTokensUpdate(ctx, readBody, sendJson, sendError);
      return;
    }
    if (url === "/tokens" && req.method === "DELETE") {
      await handleTokensDelete(ctx, readBody, sendJson, sendError);
      return;
    }

    // --- PUT /api/arts/:path/source (update art source) ---
    if (url?.startsWith("/arts/") && req.method === "PUT") {
      const rest = url.slice(6);
      const sourceMatch = rest.match(/^(.+)\/source$/);
      if (sourceMatch) {
        const artPath = decodeURIComponent(sourceMatch[1]);
        const art = ctx.artFiles.get(artPath);
        if (!art) {
          sendError("Art not found", 404);
          return;
        }

        let body = "";
        req.on("data", (chunk: string) => {
          body += chunk;
        });
        req.on("end", async () => {
          try {
            const { source } = JSON.parse(body) as { source: string };
            if (typeof source !== "string") {
              sendError("Missing required field: source", 400);
              return;
            }
            await fs.promises.writeFile(artPath, source, "utf-8");
            await ctx.processArtFile(artPath);
            sendJson({ success: true });
          } catch (e) {
            sendError(e instanceof Error ? e.message : String(e));
          }
        });
        return;
      }
      next();
      return;
    }

    // --- GET /api/arts/:path/... sub-routes ---
    if (url?.startsWith("/arts/") && req.method === "GET") {
      const rest = url.slice(6);

      const sourceMatch = rest.match(/^(.+)\/source$/);
      const paletteMatch = rest.match(/^(.+)\/palette$/);
      const analysisMatch = rest.match(/^(.+)\/analysis$/);
      const docsMatch = rest.match(/^(.+)\/docs$/);
      const a11yMatch = rest.match(/^(.+)\/variants\/([^/]+)\/a11y$/);

      if (sourceMatch) {
        await handleArtSource(ctx, sourceMatch, sendJson, sendError);
        return;
      }

      if (paletteMatch) {
        await handleArtPalette(ctx, paletteMatch, sendJson, sendError);
        return;
      }

      if (analysisMatch) {
        await handleArtAnalysis(ctx, analysisMatch, sendJson, sendError);
        return;
      }

      if (docsMatch) {
        await handleArtDocs(ctx, docsMatch, sendJson, sendError);
        return;
      }

      if (a11yMatch) {
        handleArtA11y(ctx, a11yMatch, sendJson, sendError);
        return;
      }

      // GET /api/arts/:path (no sub-resource)
      const artPath = decodeURIComponent(rest);
      const art = ctx.artFiles.get(artPath);
      if (art) {
        sendJson(art);
      } else {
        sendError("Art not found", 404);
      }
      return;
    }

    // --- POST /api/preview-with-props ---
    if (url === "/preview-with-props" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", () => {
        try {
          const { artPath: reqArtPath, variantName, props: propsOverride } = JSON.parse(body);
          const art = ctx.artFiles.get(reqArtPath);
          if (!art) {
            sendError("Art not found", 404);
            return;
          }

          const variant = art.variants.find((v) => v.name === variantName);
          if (!variant) {
            sendError("Variant not found", 404);
            return;
          }

          const variantComponentName = toPascalCase(variant.name);
          const moduleCode = generatePreviewModuleWithProps(
            art,
            variantComponentName,
            variant.name,
            propsOverride,
            ctx.resolvedPreviewCss,
            ctx.resolvedPreviewSetup,
          );
          res.setHeader("Content-Type", "application/javascript");
          res.end(moduleCode);
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    // --- POST /api/generate ---
    if (url === "/generate" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", async () => {
        try {
          const { componentPath: reqComponentPath, options: autogenOptions } = JSON.parse(body);
          const { generateArtFile: genArt } = await import("../autogen/index.js");
          const result = await genArt(reqComponentPath, autogenOptions);
          sendJson({
            generated: true,
            componentName: result.componentName,
            variants: result.variants,
            artFileContent: result.artFileContent,
          });
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    // --- POST /api/run-vrt ---
    if (url === "/run-vrt" && req.method === "POST") {
      let body = "";
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", async () => {
        try {
          const { artPath, updateSnapshots } = JSON.parse(body);
          const { MuseaVrtRunner } = await import("../vrt.js");

          const runner = new MuseaVrtRunner({
            snapshotDir: path.resolve(ctx.config.root, ".vize/snapshots"),
          });

          const port = ctx.getDevServerPort();
          const baseUrl = `http://localhost:${port}`;

          let artsToTest = Array.from(ctx.artFiles.values());
          if (artPath) {
            artsToTest = artsToTest.filter((a) => a.path === artPath);
          }

          await runner.start();
          const results = await runner.runTests(artsToTest, baseUrl, {
            updateSnapshots,
          });
          const summary = runner.getSummary(results);
          await runner.stop();

          sendJson({
            success: true,
            summary,
            results: results.map((r) => ({
              artPath: r.artPath,
              variantName: r.variantName,
              viewport: r.viewport.name,
              passed: r.passed,
              isNew: r.isNew,
              diffPercentage: r.diffPercentage,
              error: r.error,
            })),
          });
        } catch (e) {
          sendError(e instanceof Error ? e.message : String(e));
        }
      });
      return;
    }

    next();
  };
}
