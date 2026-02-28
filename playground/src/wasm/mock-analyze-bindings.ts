// Binding extraction helpers for mock SFC analysis

import type {
  BindingDisplay,
  BindingSource,
  MacroDisplay,
  PropDisplay,
  EmitDisplay,
  ProvideDisplay,
  InjectDisplay,
} from './types';

// Extract bindings (ref, computed, function) from source
export function extractBindings(
  source: string,
  isUsedInTemplate: (name: string) => boolean,
): BindingDisplay[] {
  const bindings: BindingDisplay[] = [];

  // Extract ref bindings
  const refMatches = source.matchAll(/const\s+(\w+)\s*=\s*ref\(/g);
  for (const match of refMatches) {
    const name = match[1];
    const usedInTpl = isUsedInTemplate(name);
    bindings.push({
      name,
      kind: 'SetupRef',
      source: 'ref' as BindingSource,
      metadata: {
        isExported: false,
        isImported: false,
        isComponent: false,
        isDirective: false,
        needsValue: true,
        usedInTemplate: usedInTpl,
        usedInScript: true,
        scopeDepth: 0,
      },
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
      isUsed: true,
      isMutated: true,
      referenceCount: 1,
      bindable: true,
      usedInTemplate: usedInTpl,
      fromScriptSetup: true,
    });
  }

  // Extract computed bindings
  const computedMatches = source.matchAll(/const\s+(\w+)\s*=\s*computed\(/g);
  for (const match of computedMatches) {
    const name = match[1];
    const usedInTpl = isUsedInTemplate(name);
    bindings.push({
      name,
      kind: 'SetupComputed',
      source: 'computed' as BindingSource,
      metadata: {
        isExported: false,
        isImported: false,
        isComponent: false,
        isDirective: false,
        needsValue: true,
        usedInTemplate: usedInTpl,
        usedInScript: true,
        scopeDepth: 0,
      },
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
      isUsed: true,
      isMutated: false,
      referenceCount: 1,
      bindable: true,
      usedInTemplate: usedInTpl,
      fromScriptSetup: true,
    });
  }

  // Extract function bindings
  const functionMatches = source.matchAll(/function\s+(\w+)\s*\(/g);
  for (const match of functionMatches) {
    const name = match[1];
    const usedInTpl = isUsedInTemplate(name);
    bindings.push({
      name,
      kind: 'SetupConst',
      source: 'function' as BindingSource,
      metadata: {
        isExported: false,
        isImported: false,
        isComponent: false,
        isDirective: false,
        needsValue: false,
        usedInTemplate: usedInTpl,
        usedInScript: true,
        scopeDepth: 0,
      },
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
      isUsed: true,
      isMutated: false,
      referenceCount: 1,
      bindable: true,
      usedInTemplate: usedInTpl,
      fromScriptSetup: true,
    });
  }

  return bindings;
}

// Extract defineProps macro
export function extractDefineProps(source: string): { macros: MacroDisplay[], props: PropDisplay[] } {
  const macros: MacroDisplay[] = [];
  const props: PropDisplay[] = [];

  const hasDefineProps = source.includes('defineProps');
  if (hasDefineProps) {
    const propsMatch = source.match(/defineProps<([^>]+)>\s*\(\s*\)/);
    if (propsMatch) {
      const typeArg = propsMatch[1].trim();
      macros.push({
        name: 'defineProps',
        start: propsMatch.index || 0,
        end: (propsMatch.index || 0) + propsMatch[0].length,
        type_args: typeArg,
      });
      if (typeArg.startsWith('{')) {
        const propNameMatches = typeArg.matchAll(/(\w+)(\?)?:/g);
        for (const propMatch of propNameMatches) {
          props.push({
            name: propMatch[1],
            required: !propMatch[2],
            has_default: false,
          });
        }
      }
    }
  }

  return { macros, props };
}

// Extract defineEmits macro
export function extractDefineEmits(source: string): { macros: MacroDisplay[], emits: EmitDisplay[] } {
  const macros: MacroDisplay[] = [];
  const emits: EmitDisplay[] = [];

  const hasDefineEmits = source.includes('defineEmits');
  if (hasDefineEmits) {
    const emitsMatch = source.match(/defineEmits<([^>]+)>\s*\(\s*\)/);
    if (emitsMatch) {
      const typeArg = emitsMatch[1].trim();
      macros.push({
        name: 'defineEmits',
        start: emitsMatch.index || 0,
        end: (emitsMatch.index || 0) + emitsMatch[0].length,
        type_args: typeArg,
      });
      if (typeArg.startsWith('{')) {
        const emitNameMatches = typeArg.matchAll(/(\w+):/g);
        for (const emitMatch of emitNameMatches) {
          emits.push({
            name: emitMatch[1],
          });
        }
      }
    }
  }

  return { macros, emits };
}

// Extract provides and injects from source
export function extractProvidesAndInjects(source: string): {
  provides: ProvideDisplay[];
  injects: InjectDisplay[];
} {
  const provides: ProvideDisplay[] = [];
  const injects: InjectDisplay[] = [];

  // Extract provide() calls
  const provideMatches = source.matchAll(/provide\s*\(\s*(['"])([^'"]+)\1\s*,\s*([^)]+)\)/g);
  for (const match of provideMatches) {
    provides.push({
      key: { type: 'string', value: match[2] },
      value: match[3].trim(),
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
    });
  }

  // Extract inject() calls - simple pattern
  const injectMatches = source.matchAll(/(?:const|let)\s+(\w+)\s*=\s*inject\s*[^(]*\(\s*(['"])([^'"]+)\2(?:\s*,\s*([^)]+))?\)/g);
  for (const match of injectMatches) {
    injects.push({
      key: { type: 'string', value: match[3] },
      localName: match[1],
      defaultValue: match[4]?.trim(),
      pattern: 'simple',
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
    });
  }

  // Extract destructured inject() calls
  const destructuredInjectMatches = source.matchAll(/const\s*\{\s*([^}]+)\s*\}\s*=\s*inject\s*[^(]*\(\s*(['"])([^'"]+)\2(?:\s*,\s*([^)]+))?\)/g);
  for (const match of destructuredInjectMatches) {
    const destructuredProps = match[1].split(',').map(p => p.trim().split(':')[0].trim());
    injects.push({
      key: { type: 'string', value: match[3] },
      localName: `{${destructuredProps.join(', ')}}`,
      defaultValue: match[4]?.trim(),
      pattern: 'objectDestructure',
      destructuredProps,
      start: match.index || 0,
      end: (match.index || 0) + match[0].length,
    });
  }

  return { provides, injects };
}

// Check if a binding name is used in template content
export function createTemplateUsageChecker(templateContent: string): (name: string) => boolean {
  return (name: string): boolean => {
    const patterns = [
      new RegExp(`\\{\\{[^}]*\\b${name}\\b[^}]*\\}\\}`, 'g'),
      new RegExp(`:[a-z-]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),
      new RegExp(`@[a-z-]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),
      new RegExp(`v-[a-z]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),
    ];
    return patterns.some(p => p.test(templateContent));
  };
}
