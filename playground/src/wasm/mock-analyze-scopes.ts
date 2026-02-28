// Mock scope detection for SFC analysis

import type {
  ScopeDisplay,
  ScopeKind,
  TypeExportDisplay,
  BindingDisplay,
} from './types';
import {
  extractDeclarations,
  extractImports,
  JSU_GLOBALS,
  JSS_GLOBALS,
  CLIENT_GLOBALS,
  VUE_GLOBALS,
} from './mock-analyze-helpers';
import { detectSetupScopes } from './mock-analyze-scopes-setup';

// Build all scopes for an SFC source
export function buildScopes(
  source: string,
  bindings: BindingDisplay[],
  typeExports: TypeExportDisplay[],
): ScopeDisplay[] {
  const scopes: ScopeDisplay[] = [];
  let scopeId = 0;
  const hasScriptSetup = source.includes('<script setup');
  const hoistedBindings: string[] = [];

  // Module scope (root)
  const moduleScope: ScopeDisplay = {
    id: scopeId++,
    kind: 'mod',
    kindStr: 'Mod',
    start: 0,
    end: source.length,
    bindings: [],
    children: [],
    depth: 0,
  };
  scopes.push(moduleScope);

  // Detect non-script-setup block
  const nonSetupScriptMatch = source.match(/<script(?![^>]*setup)[^>]*>([\s\S]*?)<\/script>/);
  if (nonSetupScriptMatch) {
    const nonSetupStart = source.indexOf(nonSetupScriptMatch[0]);
    const nonSetupEnd = nonSetupStart + nonSetupScriptMatch[0].length;
    const nonSetupContent = nonSetupScriptMatch[1];
    const nonSetupContentStart = nonSetupStart + nonSetupScriptMatch[0].indexOf('>') + 1;

    const { names: importNames, externalImports } = extractImports(nonSetupContent, nonSetupContentStart);
    const declNames = extractDeclarations(nonSetupContent);
    const allPlainBindings = [...new Set([...importNames, ...declNames])];
    hoistedBindings.push(...allPlainBindings);

    const nonSetupScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [0],
      kind: 'plain' as ScopeKind,
      kindStr: 'Plain',
      start: nonSetupStart,
      end: nonSetupEnd,
      bindings: allPlainBindings,
      children: [],
      depth: 1,
    };
    moduleScope.children.push(nonSetupScope.id);
    scopes.push(nonSetupScope);

    for (const ext of externalImports) {
      const externalScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [nonSetupScope.id],
        kind: 'extern' as ScopeKind,
        kindStr: `Extern (${ext.path})`,
        start: ext.start,
        end: ext.end,
        bindings: ext.name.split(', ').filter(Boolean),
        children: [],
        depth: 2,
      };
      nonSetupScope.children.push(externalScope.id);
      scopes.push(externalScope);
    }
  }

  // Detect setup scope
  if (hasScriptSetup) {
    const scriptSetupMatch = source.match(/<script[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
    if (scriptSetupMatch) {
      const setupStart = source.indexOf(scriptSetupMatch[0]);
      const setupEnd = setupStart + scriptSetupMatch[0].length;
      const setupContent = scriptSetupMatch[1];
      const contentStart = setupStart + scriptSetupMatch[0].indexOf('>') + 1;

      const { names: setupImportNames, externalImports: setupExternalImports } = extractImports(setupContent, contentStart);
      hoistedBindings.push(...setupImportNames);
      hoistedBindings.push(...typeExports.filter(t => t.hoisted).map(t => t.name));

      const setupBindings = bindings.map(b => b.name);

      const setupScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [0],
        kind: 'setup' as ScopeKind,
        kindStr: 'Setup',
        start: setupStart,
        end: setupEnd,
        bindings: setupBindings,
        children: [],
        depth: 1,
      };
      moduleScope.children.push(setupScope.id);
      scopes.push(setupScope);

      for (const ext of setupExternalImports) {
        const externalScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [setupScope.id],
          kind: 'extern' as ScopeKind,
          kindStr: `Extern (${ext.path})`,
          start: ext.start,
          end: ext.end,
          bindings: ext.name.split(', ').filter(Boolean),
          children: [],
          depth: 2,
        };
        setupScope.children.push(externalScope.id);
        scopes.push(externalScope);
      }

      // Detect scopes inside setup
      const { scopes: setupScopes, nextScopeId } = detectSetupScopes(
        setupContent, contentStart, setupScope.id, scopeId
      );
      scopeId = nextScopeId;
      for (const s of setupScopes) {
        setupScope.children.push(s.id);
        scopes.push(s);
      }
    }
  }

  // Detect v-for scopes in template
  const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
  if (templateMatch) {
    const templateTagStart = source.indexOf(templateMatch[0]);
    const templateContentStart = templateTagStart + templateMatch[0].indexOf('>') + 1;
    const vForRegex = /v-for="([^"]+)"/g;
    let vForMatch;
    while ((vForMatch = vForRegex.exec(templateMatch[1])) !== null) {
      const expr = vForMatch[1];
      const inMatch = expr.match(/\(?([^)]+)\)?\s+(?:in|of)\s+/);
      const vForBindings: string[] = [];
      if (inMatch) {
        const aliases = inMatch[1].split(',').map(s => s.trim());
        vForBindings.push(...aliases);
      }

      const vForScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [0],
        kind: 'vFor',
        kindStr: `v-for`,
        start: templateContentStart + vForMatch.index,
        end: templateContentStart + vForMatch.index + vForMatch[0].length,
        bindings: vForBindings,
        children: [],
        depth: 1,
      };
      (vForScope as any)._isTemplateScope = true;
      moduleScope.children.push(vForScope.id);
      scopes.push(vForScope);
    }

    // Detect v-slot scopes
    const vSlotRegex = /v-slot(?::(\w+))?="([^"]+)"/g;
    let vSlotMatch;
    while ((vSlotMatch = vSlotRegex.exec(templateMatch[1])) !== null) {
      const slotName = vSlotMatch[1] || 'default';
      const slotParams = vSlotMatch[2]?.match(/\{?\s*([^}]+)\s*\}?/)?.[1]?.split(',').map(s => s.trim()) || [];
      const vSlotScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [0],
        kind: 'vSlot',
        kindStr: `v-slot:${slotName}`,
        start: templateContentStart + vSlotMatch.index,
        end: templateContentStart + vSlotMatch.index + vSlotMatch[0].length,
        bindings: slotParams,
        children: [],
        depth: 1,
      };
      (vSlotScope as any)._isTemplateScope = true;
      moduleScope.children.push(vSlotScope.id);
      scopes.push(vSlotScope);
    }

    // Detect inline event handler scopes
    const eventRegex = /@(\w+)="([^"]+)"/g;
    let eventMatch;
    while ((eventMatch = eventRegex.exec(templateMatch[1])) !== null) {
      const handler = eventMatch[2];
      if (handler.includes('=>') || handler.includes('$event')) {
        const eventScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],
          kind: 'arrowFunction',
          kindStr: `@${eventMatch[1]} handler`,
          start: templateContentStart + eventMatch.index,
          end: templateContentStart + eventMatch.index + eventMatch[0].length,
          bindings: ['$event'],
          children: [],
          depth: 1,
        };
        (eventScope as any)._isTemplateScope = true;
        moduleScope.children.push(eventScope.id);
        scopes.push(eventScope);
      }
    }
  }

  // Add global scopes
  const universalScope: ScopeDisplay = {
    id: scopeId++,
    parentIds: [],
    kind: 'universal' as ScopeKind,
    kindStr: 'JsGlobal',
    start: 0,
    end: 0,
    bindings: JSU_GLOBALS,
    children: [],
    depth: 0,
  };
  scopes.unshift(universalScope);

  const vueScope: ScopeDisplay = {
    id: scopeId++,
    parentIds: [],
    kind: 'vue' as ScopeKind,
    kindStr: 'Vue',
    start: 0,
    end: 0,
    bindings: VUE_GLOBALS,
    children: [],
    depth: 0,
  };
  scopes.splice(1, 0, vueScope);

  const serverScope: ScopeDisplay = {
    id: scopeId++,
    parentIds: [],
    kind: 'server' as ScopeKind,
    kindStr: 'Server',
    start: 0,
    end: 0,
    bindings: JSS_GLOBALS,
    children: [],
    depth: 0,
  };
  scopes.push(serverScope);

  const clientScope: ScopeDisplay = {
    id: scopeId++,
    parentIds: [],
    kind: 'client' as ScopeKind,
    kindStr: 'Client',
    start: 0,
    end: 0,
    bindings: CLIENT_GLOBALS,
    children: [],
    depth: 0,
  };
  scopes.push(clientScope);

  // Add vue_global as parent for template scopes
  for (const scope of scopes) {
    if ((scope as any)._isTemplateScope && scope.parentIds) {
      scope.parentIds.push(vueScope.id);
      delete (scope as any)._isTemplateScope;
    }
  }

  // Populate module scope bindings with hoisted items + jsu globals
  moduleScope.bindings = [...new Set([...hoistedBindings, ...JSU_GLOBALS])];

  return scopes;
}
