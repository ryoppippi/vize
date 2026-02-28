import type { CrossFileIssue, AnalyzerContext } from "../types";
import { createIssue, stripComments, findLineAndColumnAtOffset } from "../utils";

const NATIVE_EVENTS = new Set([
  "click",
  "dblclick",
  "mousedown",
  "mouseup",
  "mousemove",
  "mouseenter",
  "mouseleave",
  "keydown",
  "keyup",
  "keypress",
  "focus",
  "blur",
  "change",
  "input",
  "submit",
  "scroll",
  "resize",
  "load",
  "error",
  "contextmenu",
  "wheel",
  "touchstart",
  "touchmove",
  "touchend",
  "drag",
  "dragstart",
  "dragend",
  "drop",
]);

function isNativeEvent(event: string): boolean {
  return NATIVE_EVENTS.has(event);
}

export function analyzeComponentEmits(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(ctx.files)) {
    const result = ctx.croquisResults[filename];
    const codeOnly = stripComments(source);

    const declaredEmits: Array<{
      name: string;
      loc: { line: number; column: number; endLine: number; endColumn: number };
    }> = [];

    if (result?.croquis?.emits) {
      for (const emit of result.croquis.emits) {
        const defineEmitsMacro = result.croquis.macros?.find((m) => m.name === "defineEmits");
        if (defineEmitsMacro) {
          const loc = findLineAndColumnAtOffset(
            source,
            defineEmitsMacro.start,
            defineEmitsMacro.end - defineEmitsMacro.start,
          );
          declaredEmits.push({ name: emit.name, loc });
        }
      }
    } else {
      const emitDeclRegex = /defineEmits\s*<\s*\{([^}]+)\}\s*>/s;
      const emitDeclMatch = emitDeclRegex.exec(source);
      if (emitDeclMatch && emitDeclMatch.index !== undefined) {
        const emitContent = emitDeclMatch[1];
        const emitContentOffset = emitDeclMatch.index + emitDeclMatch[0].indexOf(emitContent);

        const shorthandRegex = /(\w+)\s*:\s*\[/g;
        let match;
        while ((match = shorthandRegex.exec(emitContent)) !== null) {
          const absoluteOffset = emitContentOffset + match.index;
          const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[1].length);
          declaredEmits.push({ name: match[1], loc });
        }

        const callbackRegex = /\(\s*e:\s*['"]([^'"]+)['"]/g;
        while ((match = callbackRegex.exec(emitContent)) !== null) {
          const absoluteOffset = emitContentOffset + match.index;
          const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[0].length);
          declaredEmits.push({ name: match[1], loc });
        }
      }
    }

    if (declaredEmits.length === 0) continue;

    for (const emit of declaredEmits) {
      const emitCallRegex = new RegExp(`emit\\s*\\(\\s*['"]${emit.name}['"]`, "g");
      if (!emitCallRegex.test(codeOnly)) {
        issues.push(
          createIssue(
            "component-emit",
            "cross-file/unused-emit",
            "warning",
            `Event '${emit.name}' is declared in defineEmits but never emitted`,
            filename,
            emit.loc.line,
            emit.loc.column,
            {
              endLine: emit.loc.endLine,
              endColumn: emit.loc.endColumn,
              suggestion: `Remove '${emit.name}' from defineEmits if not needed`,
            },
          ),
        );
      }
    }

    const emitCallRegex = /emit\s*\(\s*['"]([^'"]+)['"]/g;
    let match;
    while ((match = emitCallRegex.exec(codeOnly)) !== null) {
      const emitName = match[1];
      if (!declaredEmits.some((e) => e.name === emitName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(
          createIssue(
            "component-emit",
            "cross-file/undeclared-emit",
            "error",
            `Event '${emitName}' is emitted but not declared in defineEmits`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: `Add '${emitName}' to defineEmits type definition`,
            },
          ),
        );
      }
    }
  }

  for (const [filename, source] of Object.entries(ctx.files)) {
    const listenerRegex = /@([\w-]+)(?:\.[\w-]+)*="/g;
    let match;
    while ((match = listenerRegex.exec(source)) !== null) {
      const eventName = match[1];
      if (isNativeEvent(eventName)) continue;

      const imports = ctx.dependencyGraph[filename] || [];
      let hasEmitter = false;
      for (const imp of imports) {
        const impSource = ctx.files[imp];
        if (impSource && impSource.includes(`'${eventName}'`)) {
          hasEmitter = true;
          break;
        }
      }

      if (!hasEmitter && !["update", "modelValue"].includes(eventName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(
          createIssue(
            "component-emit",
            "cross-file/unmatched-listener",
            "info",
            `Listening for @${eventName} but no imported component declares this emit`,
            filename,
            loc.line,
            loc.column,
            { endLine: loc.endLine, endColumn: loc.endColumn },
          ),
        );
      }
    }
  }

  return issues;
}
