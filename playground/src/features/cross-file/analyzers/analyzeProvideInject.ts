import type { CrossFileIssue, AnalyzerContext } from "../types";
import { createIssue, findLineAndColumnAtOffset } from "../utils";

export function analyzeProvideInject(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const provides: Map<
    string,
    {
      file: string;
      line: number;
      column: number;
      endLine: number;
      endColumn: number;
      isSymbol: boolean;
    }
  > = new Map();
  const injects: Array<{
    key: string;
    file: string;
    line: number;
    column: number;
    endLine: number;
    endColumn: number;
    hasDefault: boolean;
    isSymbol: boolean;
    pattern?: string;
    destructuredProps?: string[];
  }> = [];

  for (const [filename, source] of Object.entries(ctx.files)) {
    const result = ctx.croquisResults[filename];
    if (!result?.croquis) continue;

    for (const p of result.croquis.provides || []) {
      const keyValue = p.key.type === "symbol" ? `Symbol:${p.key.value}` : p.key.value;
      const loc = findLineAndColumnAtOffset(source, p.start, p.end - p.start);
      provides.set(keyValue, { file: filename, isSymbol: p.key.type === "symbol", ...loc });
    }

    for (const i of result.croquis.injects || []) {
      const keyValue = i.key.type === "symbol" ? `Symbol:${i.key.value}` : i.key.value;
      const loc = findLineAndColumnAtOffset(source, i.start, i.end - i.start);
      injects.push({
        key: keyValue,
        file: filename,
        hasDefault: !!i.defaultValue,
        isSymbol: i.key.type === "symbol",
        pattern: i.pattern,
        destructuredProps: i.destructuredProps,
        ...loc,
      });
    }
  }

  for (const inject of injects) {
    if (inject.pattern === "objectDestructure" || inject.pattern === "arrayDestructure") {
      const displayKey = inject.isSymbol ? inject.key.replace("Symbol:", "") : `'${inject.key}'`;
      const propsStr = inject.destructuredProps?.join(", ") || "";
      issues.push(
        createIssue(
          "provide-inject",
          "cross-file/destructured-inject",
          "error",
          `Destructuring inject(${displayKey}) into { ${propsStr} } breaks reactivity`,
          inject.file,
          inject.line,
          inject.column,
          {
            endLine: inject.endLine,
            endColumn: inject.endColumn,
            suggestion: `Store inject result first, then access properties: const data = inject(${displayKey})`,
          },
        ),
      );
    }
  }

  for (const inject of injects) {
    if (!provides.has(inject.key)) {
      const severity = inject.hasDefault ? "info" : "warning";
      const displayKey = inject.isSymbol ? inject.key.replace("Symbol:", "") : `'${inject.key}'`;
      const provideExample = inject.isSymbol
        ? `Add provide(${inject.key.replace("Symbol:", "")}, value) in a parent component`
        : `Add provide('${inject.key}', value) in a parent component`;
      issues.push(
        createIssue(
          "provide-inject",
          "cross-file/unmatched-inject",
          severity,
          `inject(${displayKey}) has no matching provide() in any ancestor component`,
          inject.file,
          inject.line,
          inject.column,
          {
            endLine: inject.endLine,
            endColumn: inject.endColumn,
            suggestion: inject.hasDefault
              ? "Using default value since no provider found"
              : provideExample,
          },
        ),
      );
    }
  }

  for (const [key, loc] of provides.entries()) {
    const hasConsumer = injects.some((i) => i.key === key);
    if (!hasConsumer) {
      const displayKey = key.startsWith("Symbol:") ? key.replace("Symbol:", "") : `'${key}'`;
      issues.push(
        createIssue(
          "provide-inject",
          "cross-file/unused-provide",
          "info",
          `provide(${displayKey}) is not consumed by any descendant component`,
          loc.file,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: "Remove if not needed, or add inject() in a child component",
          },
        ),
      );
    }
  }

  return issues;
}
