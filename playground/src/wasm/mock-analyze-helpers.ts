// Shared helpers for mock SFC analysis

import type {
  TypeExportDisplay,
  InvalidExportDisplay,
} from './types';

// Re-export split modules for backward compatibility
export { extractBindings, extractDefineProps, extractDefineEmits, extractProvidesAndInjects, createTemplateUsageChecker } from './mock-analyze-bindings';
export { generateVir } from './mock-analyze-vir';

// Helper function to strip comments from code (for accurate parsing)
export function stripComments(code: string): string {
  // Remove single-line comments
  let result = code.replace(/\/\/.*$/gm, '');
  // Remove multi-line comments
  result = result.replace(/\/\*[\s\S]*?\*\//g, '');
  return result;
}

// Helper function to extract declarations (functions, variables) from script content
export function extractDeclarations(content: string): string[] {
  const stripped = stripComments(content);
  const names: string[] = [];
  // Functions
  const funcRegex = /(?:export\s+)?(?:async\s+)?function\s+(\w+)/g;
  let match;
  while ((match = funcRegex.exec(stripped)) !== null) {
    names.push(match[1]);
  }
  // const/let/var declarations
  const varRegex = /(?:export\s+)?(?:const|let|var)\s+(\w+)/g;
  while ((match = varRegex.exec(stripped)) !== null) {
    names.push(match[1]);
  }
  return names;
}

// Import info type
export interface ImportInfo {
  name: string;
  path: string;
  start: number;
  end: number;
}

// Helper function to extract imports from script content
export function extractImports(content: string, startOffset: number): { names: string[], externalImports: ImportInfo[] } {
  const stripped = stripComments(content);
  const names: string[] = [];
  const externalImports: ImportInfo[] = [];
  const importRegex = /import\s+(?:type\s+)?(?:(\w+)|{\s*([^}]+)\s*}|\*\s+as\s+(\w+))?\s*(?:,\s*{\s*([^}]+)\s*})?\s*from\s+['"]([^'"]+)['"]/g;
  let match;
  while ((match = importRegex.exec(stripped)) !== null) {
    const defaultImport = match[1];
    const namedImports = match[2];
    const namespaceImport = match[3];
    const additionalNamed = match[4];
    const modulePath = match[5];

    const importedNames: string[] = [];
    if (defaultImport) importedNames.push(defaultImport);
    if (namespaceImport) importedNames.push(namespaceImport);
    if (namedImports) {
      const parsed = namedImports.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
      importedNames.push(...parsed);
    }
    if (additionalNamed) {
      const parsed = additionalNamed.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
      importedNames.push(...parsed);
    }

    names.push(...importedNames);

    // Check if it's an external module (not relative path or alias)
    const isExternal = !modulePath.startsWith('.') && !modulePath.startsWith('@/');
    if (isExternal) {
      externalImports.push({
        name: importedNames.join(', ') || modulePath,
        path: modulePath,
        start: startOffset + match.index,
        end: startOffset + match.index + match[0].length,
      });
    }
  }
  return { names, externalImports };
}

// JS universal globals (available everywhere in both server and client)
export const JSU_GLOBALS = [
  'console', 'Math', 'JSON', 'Date', 'Array', 'Object', 'String', 'Number',
  'Boolean', 'Symbol', 'BigInt', 'Map', 'Set', 'WeakMap', 'WeakSet',
  'Promise', 'Proxy', 'Reflect', 'Error', 'TypeError', 'RangeError',
  'parseInt', 'parseFloat', 'isNaN', 'isFinite', 'encodeURI', 'decodeURI',
  'encodeURIComponent', 'decodeURIComponent', 'undefined', 'NaN', 'Infinity',
];

// JS server-only globals (Node.js)
export const JSS_GLOBALS = [
  'process', 'Buffer', '__dirname', '__filename', 'module', 'exports', 'require',
  'global', 'setImmediate', 'clearImmediate',
];

// JS client-only globals (Browser)
export const CLIENT_GLOBALS = [
  'window', 'document', 'navigator', 'location', 'history', 'localStorage',
  'sessionStorage', 'fetch', 'XMLHttpRequest', 'WebSocket', 'Worker',
  'requestAnimationFrame', 'cancelAnimationFrame', 'setTimeout', 'clearTimeout',
  'setInterval', 'clearInterval', 'alert', 'confirm', 'prompt',
];

// Vue globals (template-only)
export const VUE_GLOBALS = [
  '$refs', '$emit', '$attrs', '$slots', '$props', '$el', '$options',
  '$data', '$watch', '$nextTick', '$forceUpdate',
];

// Extract type exports and invalid exports from source
export function extractExports(source: string): {
  typeExports: TypeExportDisplay[];
  invalidExports: InvalidExportDisplay[];
} {
  const typeExports: TypeExportDisplay[] = [];
  const invalidExports: InvalidExportDisplay[] = [];
  const hasScriptSetup = source.includes('<script setup');

  // Extract type exports (export type / export interface) - valid in script setup
  const typeExportRegex = /export\s+type\s+(\w+)\s*=/g;
  let typeMatch;
  while ((typeMatch = typeExportRegex.exec(source)) !== null) {
    typeExports.push({
      name: typeMatch[1],
      kind: 'type',
      start: typeMatch.index,
      end: typeMatch.index + typeMatch[0].length,
      hoisted: true,
    });
  }

  const interfaceExportRegex = /export\s+interface\s+(\w+)\s*\{/g;
  while ((typeMatch = interfaceExportRegex.exec(source)) !== null) {
    typeExports.push({
      name: typeMatch[1],
      kind: 'interface',
      start: typeMatch.index,
      end: typeMatch.index + typeMatch[0].length,
      hoisted: true,
    });
  }

  // Extract invalid exports in script setup
  if (hasScriptSetup) {
    const scriptSetupMatch = source.match(/<script[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
    if (scriptSetupMatch) {
      const setupContent = scriptSetupMatch[1];
      const setupStart = source.indexOf(scriptSetupMatch[0]) + scriptSetupMatch[0].indexOf('>') + 1;

      const patterns: Array<{ regex: RegExp; kind: InvalidExportDisplay['kind']; getName: (m: RegExpExecArray) => string }> = [
        { regex: /export\s+const\s+(\w+)/g, kind: 'const', getName: m => m[1] },
        { regex: /export\s+let\s+(\w+)/g, kind: 'let', getName: m => m[1] },
        { regex: /export\s+var\s+(\w+)/g, kind: 'var', getName: m => m[1] },
        { regex: /export\s+(?:async\s+)?function\s+(\w+)/g, kind: 'function', getName: m => m[1] },
        { regex: /export\s+class\s+(\w+)/g, kind: 'class', getName: m => m[1] },
        { regex: /export\s+default\s+/g, kind: 'default', getName: () => 'default' },
      ];

      for (const { regex, kind, getName } of patterns) {
        let exportMatch;
        while ((exportMatch = regex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: getName(exportMatch),
            kind,
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
            message: '',
          });
        }
      }
    }
  }

  return { typeExports, invalidExports };
}
