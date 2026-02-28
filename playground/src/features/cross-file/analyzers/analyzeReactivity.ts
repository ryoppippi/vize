import type { CrossFileIssue, AnalyzerContext } from "../types";
import { createIssue, stripComments, findLineAndColumnAtOffset } from "../utils";

export function analyzeReactivity(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  const provideValueIsReactive = new Map<string, boolean>();
  for (const [filename, source] of Object.entries(ctx.files)) {
    const result = ctx.croquisResults[filename];
    if (!result?.croquis) continue;

    for (const p of result.croquis.provides || []) {
      const keyValue = p.key.type === "symbol" ? `Symbol:${p.key.value}` : p.key.value;
      const valueContainsRef =
        p.value &&
        (p.value.includes("ref(") ||
          p.value.includes("reactive(") ||
          /\{\s*\w+\s*:\s*ref\s*\(/.test(p.value));
      provideValueIsReactive.set(keyValue, valueContainsRef);
    }
  }

  for (const [filename, source] of Object.entries(ctx.files)) {
    const result = ctx.croquisResults[filename];
    if (!result?.croquis) continue;

    const injectBindings = new Map<string, string>();
    for (const inj of result.croquis.injects || []) {
      if (inj.localName && inj.pattern === "simple") {
        const keyValue = inj.key.type === "symbol" ? `Symbol:${inj.key.value}` : inj.key.value;
        injectBindings.set(inj.localName, keyValue);
      }
    }

    for (const [injectVar, injectKey] of injectBindings) {
      if (provideValueIsReactive.get(injectKey)) continue;

      const destructureRegex = new RegExp(
        `const\\s*\\{\\s*([^}]+)\\s*\\}\\s*=\\s*${injectVar}\\b`,
        "g",
      );
      let match;
      while ((match = destructureRegex.exec(source)) !== null) {
        const propsStr = match[1];
        const props = propsStr
          .split(",")
          .map((p) => p.trim().split(":")[0].trim())
          .filter(Boolean);
        const matchStart = match.index;
        const loc = findLineAndColumnAtOffset(source, matchStart, match[0].length);

        issues.push(
          createIssue(
            "reactivity",
            "cross-file/reactivity-loss",
            "error",
            `Destructuring '${injectVar}' (from inject('${injectKey}')) loses reactivity for: ${props.join(", ")}`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: `Use toRefs(${injectVar}) or computed(() => ${injectVar}.propName)`,
            },
          ),
        );
      }
    }

    const reactiveBindings = new Set<string>();
    for (const binding of result.croquis.bindings || []) {
      if (binding.source === "reactive") {
        reactiveBindings.add(binding.name);
      }
    }

    for (const binding of result.croquis.bindings || []) {
      if (binding.source === "local" && binding.kind === "SetupConst") {
        const bindingStart = binding.start || 0;
        const lineStart = source.lastIndexOf("\n", bindingStart) + 1;
        const lineEnd = source.indexOf("\n", bindingStart);
        const line = source.substring(lineStart, lineEnd === -1 ? undefined : lineEnd);

        for (const reactiveVar of reactiveBindings) {
          if (source.includes(`toRefs(${reactiveVar})`)) continue;

          const destructurePattern = new RegExp(
            `const\\s*\\{[^}]*\\b${binding.name}\\b[^}]*\\}\\s*=\\s*${reactiveVar}\\b`,
          );
          if (destructurePattern.test(line)) {
            const loc = findLineAndColumnAtOffset(source, bindingStart, binding.name.length);
            issues.push(
              createIssue(
                "reactivity",
                "cross-file/reactivity-loss",
                "warning",
                `Destructuring reactive object '${reactiveVar}' loses reactivity for: ${binding.name}`,
                filename,
                loc.line,
                loc.column,
                {
                  endLine: loc.endLine,
                  endColumn: loc.endColumn,
                  suggestion: `Use toRefs(${reactiveVar}) to maintain reactivity`,
                },
              ),
            );
          }
        }
      }
    }

    const codeOnly = stripComments(source);
    const storeDestructureRegex = /const\s*\{([^}]+)\}\s*=\s*(\w+Store)\s*\(\s*\)/g;
    let match;
    while ((match = storeDestructureRegex.exec(codeOnly)) !== null) {
      if (codeOnly.includes(`storeToRefs(${match[2]}`)) continue;

      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      const props = match[1].split(",").map((p) => p.trim().split(":")[0].trim());
      issues.push(
        createIssue(
          "reactivity",
          "cross-file/store-reactivity-loss",
          "warning",
          `Destructuring Pinia store loses reactivity for: ${props.join(", ")}`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Use storeToRefs(${match[2]}()) for state and getters`,
          },
        ),
      );
    }
  }

  return issues;
}
