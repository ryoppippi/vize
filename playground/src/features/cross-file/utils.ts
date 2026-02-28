import type { CrossFileIssue } from "./types";

let issueIdCounter = 0;

export function resetIssueIdCounter() {
  issueIdCounter = 0;
}

export function createIssue(
  type: string,
  code: string,
  severity: "error" | "warning" | "info",
  message: string,
  file: string,
  line: number,
  column: number,
  opts?: {
    endLine?: number;
    endColumn?: number;
    suggestion?: string;
    relatedLocations?: Array<{ file: string; line: number; column: number; message: string }>;
  },
): CrossFileIssue {
  return {
    id: `issue-${++issueIdCounter}`,
    type,
    code,
    severity,
    message,
    file,
    line,
    column,
    ...opts,
  };
}

export function stripComments(source: string): string {
  let result = "";
  let i = 0;
  while (i < source.length) {
    if (source[i] === '"' || source[i] === "'" || source[i] === "`") {
      const quote = source[i];
      result += source[i++];
      while (i < source.length && source[i] !== quote) {
        if (source[i] === "\\" && i + 1 < source.length) {
          result += source[i++];
        }
        result += source[i++];
      }
      if (i < source.length) result += source[i++];
    } else if (source[i] === "/" && source[i + 1] === "/") {
      while (i < source.length && source[i] !== "\n") {
        result += " ";
        i++;
      }
    } else if (source[i] === "/" && source[i + 1] === "*") {
      result += "  ";
      i += 2;
      while (i < source.length && !(source[i] === "*" && source[i + 1] === "/")) {
        result += source[i] === "\n" ? "\n" : " ";
        i++;
      }
      if (i < source.length) {
        result += "  ";
        i += 2;
      }
    } else {
      result += source[i++];
    }
  }
  return result;
}

export function parseSuppressions(source: string): Set<number> {
  const suppressedLines = new Set<number>();
  const lines = source.split("\n");
  let pendingSuppression = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trim();
    const lineNumber = i + 1;

    const singleLineMatch = trimmedLine.match(/^\/\/\s*@vize\s+forget\s*:\s*(.+)/);
    const blockMatch = trimmedLine.match(/^\/\*\s*@vize\s+forget\s*:\s*(.+?)\s*\*\//);

    if ((singleLineMatch && singleLineMatch[1].trim()) || (blockMatch && blockMatch[1].trim())) {
      pendingSuppression = true;
    } else if (
      pendingSuppression &&
      trimmedLine &&
      !trimmedLine.startsWith("//") &&
      !trimmedLine.startsWith("/*")
    ) {
      suppressedLines.add(lineNumber);
      pendingSuppression = false;
    }
  }

  return suppressedLines;
}

export function buildSuppressionMap(files: Record<string, string>): Map<string, Set<number>> {
  const map = new Map<string, Set<number>>();
  for (const [filename, source] of Object.entries(files)) {
    map.set(filename, parseSuppressions(source));
  }
  return map;
}

export function filterSuppressedIssues(
  issues: CrossFileIssue[],
  suppressionMap: Map<string, Set<number>>,
): CrossFileIssue[] {
  return issues.filter((issue) => {
    const suppressedLines = suppressionMap.get(issue.file);
    if (!suppressedLines) return true;
    return !suppressedLines.has(issue.line);
  });
}

export function offsetToLineColumn(
  source: string,
  offset: number,
): { line: number; column: number } {
  const beforeOffset = source.substring(0, offset);
  const lines = beforeOffset.split("\n");
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

export function findLineAndColumn(
  source: string,
  pattern: RegExp | string,
): { line: number; column: number; endLine?: number; endColumn?: number } | null {
  const regex =
    typeof pattern === "string"
      ? new RegExp(pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
      : pattern;
  const match = source.match(regex);
  if (!match || match.index === undefined) return null;

  const start = offsetToLineColumn(source, match.index);
  const end = offsetToLineColumn(source, match.index + match[0].length);

  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

export function findLineAndColumnAtOffset(
  source: string,
  offset: number,
  length: number,
): { line: number; column: number; endLine: number; endColumn: number } {
  const start = offsetToLineColumn(source, offset);
  const end = offsetToLineColumn(source, offset + length);
  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}
