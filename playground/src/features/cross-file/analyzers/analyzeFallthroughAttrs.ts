import type { CrossFileIssue, AnalyzerContext } from "../types";
import { createIssue, findLineAndColumn, findLineAndColumnAtOffset } from "../utils";

function analyzeAttrsUsage(
  source: string,
  template: string,
): {
  bindsExplicitly: boolean;
  usesUseAttrs: boolean;
  usesInTemplate: boolean;
  usedProperties: string[];
} {
  const result = {
    bindsExplicitly: false,
    usesUseAttrs: false,
    usesInTemplate: false,
    usedProperties: [] as string[],
  };

  if (/useAttrs\s*\(\s*\)/.test(source)) {
    result.usesUseAttrs = true;
    const useAttrsMatch = source.match(/(?:const|let)\s+(\w+)\s*=\s*useAttrs\s*\(\s*\)/);
    if (useAttrsMatch) {
      const varName = useAttrsMatch[1];
      const varBindPattern = new RegExp(`v-bind=["']${varName}["']|:=["']${varName}["']`);
      if (varBindPattern.test(template)) {
        result.bindsExplicitly = true;
        result.usesInTemplate = true;
      }
      const propAccessPattern = new RegExp(`${varName}\\.(\\w+)`, "g");
      let match;
      while ((match = propAccessPattern.exec(template)) !== null) {
        result.usedProperties.push(match[1]);
        result.usesInTemplate = true;
      }
    }
  }

  if (/v-bind=["']\$attrs["']|:=["']\$attrs["']/.test(template)) {
    result.bindsExplicitly = true;
    result.usesInTemplate = true;
  }

  const attrsPropertyPattern = /\$attrs\.(\w+)/g;
  let match;
  while ((match = attrsPropertyPattern.exec(template)) !== null) {
    result.usedProperties.push(match[1]);
    result.usesInTemplate = true;
  }

  if (/\{\{\s*\$attrs\b|\$attrs\s*\}\}|:\w+=['"]\$attrs\./.test(template)) {
    result.usesInTemplate = true;
  }

  return result;
}

function countRootElements(template: string): number {
  const withoutComments = template.replace(/<!--[\s\S]*?-->/g, "");

  const voidElements = new Set([
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "param",
    "source",
    "track",
    "wbr",
  ]);

  let depth = 0;
  let rootCount = 0;

  const tagRegex = /<\/?([a-zA-Z][\w-]*)[^>]*\/?>/g;
  let match;

  while ((match = tagRegex.exec(withoutComments)) !== null) {
    const fullTag = match[0];
    const tagName = match[1].toLowerCase();

    const isClosing = fullTag.startsWith("</");
    const isSelfClosing = fullTag.endsWith("/>") || voidElements.has(tagName);

    if (isClosing) {
      depth--;
    } else {
      if (depth === 0) {
        rootCount++;
      }
      if (!isSelfClosing) {
        depth++;
      }
    }
  }

  return rootCount;
}

export function analyzeFallthroughAttrs(ctx: AnalyzerContext): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(ctx.files)) {
    const templateMatch = source.match(/<template>([\s\S]*)<\/template>/);
    if (!templateMatch) continue;

    const template = templateMatch[1];

    const rootElementCount = countRootElements(template);
    const hasMultipleRoots = rootElementCount > 1;

    const attrsUsage = analyzeAttrsUsage(source, template);

    if (hasMultipleRoots && !attrsUsage.bindsExplicitly) {
      const componentName = filename.replace(".vue", "");
      let hasPassedAttrs = false;

      for (const [parentFile, parentSource] of Object.entries(ctx.files)) {
        if (parentFile === filename) continue;
        const usageRegex = new RegExp(
          `<${componentName}[^>]*(?:data-|aria-|class=|style=)[^>]*>`,
          "i",
        );
        if (usageRegex.test(parentSource)) {
          hasPassedAttrs = true;
          break;
        }
      }

      const loc = findLineAndColumn(source, /<template[^>]*>/);
      if (loc) {
        if (attrsUsage.usesUseAttrs && !attrsUsage.usesInTemplate) {
          issues.push(
            createIssue(
              "fallthrough-attrs",
              "cross-file/useattrs-not-bound",
              "warning",
              `useAttrs() is called but attrs are not bound to any element in template`,
              filename,
              loc.line,
              loc.column,
              {
                endLine: loc.endLine,
                endColumn: loc.endColumn,
                suggestion:
                  'Use v-bind="attrs" or bind specific properties like :class="attrs.class"',
              },
            ),
          );
        } else {
          issues.push(
            createIssue(
              "fallthrough-attrs",
              "cross-file/multi-root-attrs",
              hasPassedAttrs ? "warning" : "info",
              `Component has ${rootElementCount} root elements but $attrs is not explicitly bound`,
              filename,
              loc.line,
              loc.column,
              {
                endLine: loc.endLine,
                endColumn: loc.endColumn,
                suggestion: attrsUsage.usesUseAttrs
                  ? "Bind attrs from useAttrs() to the intended root element"
                  : 'Add v-bind="$attrs" to the intended root element, or use useAttrs() composable',
              },
            ),
          );
        }
      }
    }

    if (source.includes("inheritAttrs: false") || source.includes("inheritAttrs:false")) {
      if (!attrsUsage.usesInTemplate && !attrsUsage.usesUseAttrs) {
        const loc = findLineAndColumn(source, /inheritAttrs\s*:\s*false/);
        if (loc) {
          issues.push(
            createIssue(
              "fallthrough-attrs",
              "cross-file/inheritattrs-disabled-unused",
              "warning",
              `inheritAttrs is disabled but $attrs is not used anywhere`,
              filename,
              loc.line,
              loc.column,
              {
                suggestion: 'Use v-bind="$attrs", useAttrs(), or $attrs properties in template',
              },
            ),
          );
        }
      }
    }
  }

  return issues;
}
