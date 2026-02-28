import type { CrossFileIssue, AnalyzerContext } from "../types";
import { createIssue, findLineAndColumn, findLineAndColumnAtOffset } from "../utils";

export function analyzeUniqueIds(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const staticIds: Map<
    string,
    Array<{ file: string; line: number; column: number; endLine: number; endColumn: number }>
  > = new Map();

  for (const [filename, source] of Object.entries(ctx.files)) {
    const idRegex = /\bid=["']([^"'${}]+)["']/g;
    let match;
    while ((match = idRegex.exec(source)) !== null) {
      const id = match[1];
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      if (!staticIds.has(id)) staticIds.set(id, []);
      staticIds.get(id)!.push({ file: filename, ...loc });
    }

    const vforIdRegex = /v-for=[^>]+>\s*[^]*?id=["']([^"'${}]+)["']/g;
    while ((match = vforIdRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      issues.push(
        createIssue(
          "unique-id",
          "cross-file/non-unique-id",
          "error",
          `Static id="${match[1]}" inside v-for will create duplicate IDs`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: 'Use a dynamic id like :id="`item-${index}`"',
          },
        ),
      );
    }
  }

  for (const [id, locations] of staticIds.entries()) {
    if (locations.length > 1) {
      const primary = locations[0];
      issues.push(
        createIssue(
          "unique-id",
          "cross-file/duplicate-id",
          "warning",
          `Element id="${id}" is duplicated in ${locations.length} locations`,
          primary.file,
          primary.line,
          primary.column,
          {
            relatedLocations: locations.slice(1).map((loc) => ({
              file: loc.file,
              line: loc.line,
              column: loc.column,
              message: "Also defined here",
            })),
            suggestion: "Use unique IDs across your application",
          },
        ),
      );
    }
  }

  return issues;
}

export function analyzeSSRBoundary(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const browserApis = [
    "window",
    "document",
    "navigator",
    "localStorage",
    "sessionStorage",
    "location",
    "history",
  ];

  for (const [filename, source] of Object.entries(ctx.files)) {
    const scriptMatch = source.match(new RegExp("<script[^>]*>([^]*?)<" + "/script>"));
    if (!scriptMatch) continue;

    const script = scriptMatch[1];

    for (const api of browserApis) {
      const apiRegex = new RegExp(`\\b${api}\\b`, "g");
      let match;
      while ((match = apiRegex.exec(script)) !== null) {
        const beforeMatch = script.substring(0, match.index);
        const isInClientHook =
          /on(Mounted|BeforeMount|Updated|BeforeUpdate)\s*\([^)]*$/.test(beforeMatch) ||
          /onMounted\s*\(\s*(?:async\s*)?\(\)\s*=>\s*\{[^}]*$/.test(beforeMatch);

        if (!isInClientHook) {
          const scriptStart = source.indexOf(scriptMatch[1]);
          const fullOffset = scriptStart + match.index;
          const loc = findLineAndColumnAtOffset(source, fullOffset, api.length);
          issues.push(
            createIssue(
              "ssr-boundary",
              "cross-file/browser-api-ssr",
              "warning",
              `Browser API '${api}' used outside client-only lifecycle hook`,
              filename,
              loc.line,
              loc.column,
              {
                endLine: loc.endLine,
                endColumn: loc.endColumn,
                suggestion: `Move to onMounted() or guard with 'if (import.meta.client)'`,
              },
            ),
          );
        }
      }
    }
  }

  return issues;
}
