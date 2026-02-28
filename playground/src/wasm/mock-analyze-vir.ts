// Generate VIR (Vize Intermediate Representation) text

import type {
  BindingDisplay,
  MacroDisplay,
  PropDisplay,
  EmitDisplay,
  ScopeDisplay,
  TypeExportDisplay,
  InvalidExportDisplay,
} from './types';

export function generateVir(
  bindings: BindingDisplay[],
  macros: MacroDisplay[],
  props: PropDisplay[],
  emits: EmitDisplay[],
  scopes: ScopeDisplay[],
  typeExports: TypeExportDisplay[],
  invalidExports: InvalidExportDisplay[],
  hasScoped: boolean,
  source: string,
): string {
  let vir = '';
  vir += '# VIR v0.1\n';
  vir += '\n';

  // Stats section
  vir += '[stats]\n';
  vir += `bindings = ${bindings.length}\n`;
  vir += `macros = ${macros.length}\n`;

  // Macros section
  if (macros.length > 0) {
    vir += '\n[macros]\n';
    for (const macro of macros) {
      vir += `@${macro.name}`;
      if (macro.type_args) {
        vir += `<{\n${macro.type_args.trim()}\n}>`;
      }
      if ((macro as any).identifier) {
        vir += ` -> ${(macro as any).identifier}`;
      }
      vir += ` # ${macro.start}:${macro.end}\n`;
    }
  }

  // Bindings section
  if (bindings.length > 0) {
    vir += '\n[bindings]\n';
    for (const binding of bindings) {
      const flags: string[] = [];
      if (binding.kind === 'SetupRef') flags.push('ref');
      if (binding.isUsed !== false) flags.push('used');
      if (binding.isMutated) flags.push('mut');
      vir += `${binding.name}: ${binding.source} @0:0`;
      if (flags.length > 0) {
        vir += ` [${flags.join(', ')}]`;
      }
      vir += '\n';
    }
  }

  // CSS section
  const selectorCount = (source.match(/[.#\w][\w-]*\s*\{/g) || []).length;
  const vBindCount = (source.match(/v-bind\(/g) || []).length;
  if (hasScoped) {
    vir += '\n[css]\n';
    vir += `scoped = true\n`;
    vir += `selectors = ${selectorCount}\n`;
    vir += `v_bind = ${vBindCount}\n`;
  }

  // Props section
  if (props.length > 0) {
    vir += '\n[props]\n';
    for (const prop of props) {
      vir += `${prop.name}: ${(prop as any).type || 'any'}\n`;
    }
  }

  // Emits section
  if (emits.length > 0) {
    vir += '\n[emits]\n';
    for (const emit of emits) {
      vir += `${emit.name}\n`;
    }
  }

  // Get prefix for scope kind
  const getScopePrefix = (kind: string): string => {
    switch (kind) {
      case 'client': return '!';
      case 'server': return '#';
      default: return '~';
    }
  };

  // Assign display IDs per prefix type
  const prefixCounters: Record<string, number> = { '#': 0, '~': 0, '!': 0 };
  const displayIdMap = new Map<number, string>();
  for (const scope of scopes) {
    const prefix = getScopePrefix(scope.kind);
    const displayId = prefixCounters[prefix]++;
    displayIdMap.set(scope.id, `${prefix}${displayId}`);
  }

  // Get parent references
  const getParentRefs = (parentIds: number[]): string => {
    if (!parentIds || parentIds.length === 0) return '';
    const refs = parentIds.map(pid => displayIdMap.get(pid) || `#${pid}`);
    return ` $ ${refs.join(', ')}`;
  };

  // Add scopes to VIR
  if (scopes.length > 0) {
    vir += '\n[scopes]\n';
    for (const scope of scopes) {
      const displayId = displayIdMap.get(scope.id) || `#${scope.id}`;
      vir += `${displayId} ${scope.kindStr.toLowerCase()} @${scope.start}:${scope.end}`;
      if (scope.bindings.length > 0) {
        vir += ` {${scope.bindings.join(', ')}}`;
      }
      if (scope.parentIds && scope.parentIds.length > 0) {
        vir += getParentRefs(scope.parentIds);
      }
      vir += '\n';
    }
  }

  // Update stats with scope count
  vir = vir.replace('[stats]\n', `[stats]\nscopes = ${scopes.length}\n`);

  // Add type exports to VIR
  if (typeExports.length > 0) {
    vir += '\n[type_exports]\n';
    for (const te of typeExports) {
      vir += `${te.kind} ${te.name} @${te.start}:${te.end} [hoisted]\n`;
    }
  }

  // Add invalid exports to VIR
  if (invalidExports.length > 0) {
    vir += '\n[invalid_exports]\n';
    for (const ie of invalidExports) {
      vir += `${ie.kind} ${ie.name} @${ie.start}:${ie.end} [INVALID]\n`;
    }
  }

  return vir;
}
