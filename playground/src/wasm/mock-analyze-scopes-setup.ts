// Detect scopes inside a setup block's content

import type { ScopeDisplay, ScopeKind } from './types';

export function detectSetupScopes(
  setupContent: string,
  contentStart: number,
  setupScopeId: number,
  startScopeId: number,
): { scopes: ScopeDisplay[]; nextScopeId: number } {
  const scopes: ScopeDisplay[] = [];
  let scopeId = startScopeId;
  let funcMatch;

  // Detect function scopes inside setup
  const functionRegex = /function\s+(\w+)\s*\([^)]*\)\s*\{/g;
  while ((funcMatch = functionRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'function',
      kindStr: `Function (${funcMatch[1]})`,
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 50,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect arrow function scopes
  const arrowRegex = /const\s+(\w+)\s*=\s*\([^)]*\)\s*=>/g;
  while ((funcMatch = arrowRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: `Arrow (${funcMatch[1]})`,
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 50,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect watch callbacks
  const watchRegex = /watch\s*\([^,]+,\s*\(?([^)]*)\)?\s*=>/g;
  while ((funcMatch = watchRegex.exec(setupContent)) !== null) {
    const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'watch',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: params,
      children: [],
      depth: 2,
    });
  }

  // Detect watchEffect callbacks
  const watchEffectRegex = /watchEffect\s*\(\s*\(?([^)]*)\)?\s*=>/g;
  while ((funcMatch = watchEffectRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'watchEffect',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect computed callbacks
  const computedRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*(?:\([^)]*\)\s*)?=>/g;
  while ((funcMatch = computedRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: `computed (${funcMatch[1]})`,
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect computed with getter/setter
  const computedGetSetRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*\{/g;
  while ((funcMatch = computedGetSetRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'function',
      kindStr: `computed (${funcMatch[1]}) [get/set]`,
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 50,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect lifecycle hooks
  const clientOnlyHooks = ['onMounted', 'onUnmounted', 'onBeforeMount', 'onBeforeUnmount', 'onUpdated', 'onBeforeUpdate', 'onActivated', 'onDeactivated'];
  const universalHooks = ['onErrorCaptured', 'onRenderTracked', 'onRenderTriggered'];
  const serverOnlyHooks = ['onServerPrefetch'];

  for (const hook of clientOnlyHooks) {
    const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
    while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
      scopes.push({
        id: scopeId++,
        parentIds: [setupScopeId],
        kind: 'client' as ScopeKind,
        kindStr: `ClientOnly (${hook})`,
        start: contentStart + funcMatch.index,
        end: contentStart + funcMatch.index + funcMatch[0].length + 30,
        bindings: [],
        children: [],
        depth: 2,
      });
    }
  }

  for (const hook of universalHooks) {
    const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
    while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
      scopes.push({
        id: scopeId++,
        parentIds: [setupScopeId],
        kind: 'universal' as ScopeKind,
        kindStr: `Universal (${hook})`,
        start: contentStart + funcMatch.index,
        end: contentStart + funcMatch.index + funcMatch[0].length + 30,
        bindings: [],
        children: [],
        depth: 2,
      });
    }
  }

  for (const hook of serverOnlyHooks) {
    const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
    while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
      scopes.push({
        id: scopeId++,
        parentIds: [setupScopeId],
        kind: 'function',
        kindStr: `ServerOnly (${hook})`,
        start: contentStart + funcMatch.index,
        end: contentStart + funcMatch.index + funcMatch[0].length + 30,
        bindings: [],
        children: [],
        depth: 2,
      });
    }
  }

  // Detect provide with factory function
  const provideRegex = /provide\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
  while ((funcMatch = provideRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'provide factory',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 20,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect inject with default factory
  const injectRegex = /inject\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
  while ((funcMatch = injectRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'inject default',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 20,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect try-catch blocks
  const tryCatchRegex = /try\s*\{/g;
  while ((funcMatch = tryCatchRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'block',
      kindStr: 'try',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  const catchRegex = /catch\s*\(\s*(\w+)\s*\)\s*\{/g;
  while ((funcMatch = catchRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'block',
      kindStr: 'catch',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [funcMatch[1]],
      children: [],
      depth: 2,
    });
  }

  // Detect for loops
  const forLoopRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)/g;
  while ((funcMatch = forLoopRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'block',
      kindStr: 'for',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [funcMatch[1]],
      children: [],
      depth: 2,
    });
  }

  // Detect for...of / for...in loops
  const forOfInRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)\s+(?:of|in)\s+/g;
  while ((funcMatch = forOfInRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'block',
      kindStr: 'for..of/in',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [funcMatch[1]],
      children: [],
      depth: 2,
    });
  }

  // Detect if blocks with block-scoped variables
  const ifLetRegex = /if\s*\([^)]+\)\s*\{[^}]*(?:const|let)\s+(\w+)/g;
  while ((funcMatch = ifLetRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'block',
      kindStr: 'if',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 20,
      bindings: [funcMatch[1]],
      children: [],
      depth: 2,
    });
  }

  // Detect Array method callbacks
  const arrayMethodPatterns: Array<{ regex: RegExp; kindStr: string }> = [
    { regex: /\.forEach\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'forEach' },
    { regex: /\.map\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'map' },
    { regex: /\.filter\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'filter' },
    { regex: /\.reduce\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'reduce' },
    { regex: /\.find(?:Index)?\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'find' },
    { regex: /\.(?:some|every)\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: 'some/every' },
  ];

  for (const { regex, kindStr } of arrayMethodPatterns) {
    while ((funcMatch = regex.exec(setupContent)) !== null) {
      const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
      scopes.push({
        id: scopeId++,
        parentIds: [setupScopeId],
        kind: 'arrowFunction',
        kindStr,
        start: contentStart + funcMatch.index,
        end: contentStart + funcMatch.index + funcMatch[0].length + 20,
        bindings: params,
        children: [],
        depth: 2,
      });
    }
  }

  // Detect Promise callbacks
  const promisePatterns: Array<{ regex: RegExp; kindStr: string; hasParams: boolean }> = [
    { regex: /\.then\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: '.then', hasParams: true },
    { regex: /\.catch\s*\(\s*\(?([^)]*)\)?\s*=>/g, kindStr: '.catch', hasParams: true },
    { regex: /\.finally\s*\(\s*\(\)\s*=>/g, kindStr: '.finally', hasParams: false },
  ];

  for (const { regex, kindStr, hasParams } of promisePatterns) {
    while ((funcMatch = regex.exec(setupContent)) !== null) {
      const params = hasParams
        ? (funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [])
        : [];
      scopes.push({
        id: scopeId++,
        parentIds: [setupScopeId],
        kind: 'arrowFunction',
        kindStr,
        start: contentStart + funcMatch.index,
        end: contentStart + funcMatch.index + funcMatch[0].length + 20,
        bindings: params,
        children: [],
        depth: 2,
      });
    }
  }

  // Detect setTimeout/setInterval callbacks
  const timerRegex = /set(?:Timeout|Interval)\s*\(\s*\(\)\s*=>/g;
  while ((funcMatch = timerRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'timer',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 20,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect nextTick callbacks
  const nextTickRegex = /nextTick\s*\(\s*\(\)\s*=>/g;
  while ((funcMatch = nextTickRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'nextTick',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 20,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  // Detect async IIFE
  const asyncIifeRegex = /\(\s*async\s*\(\)\s*=>\s*\{/g;
  while ((funcMatch = asyncIifeRegex.exec(setupContent)) !== null) {
    scopes.push({
      id: scopeId++,
      parentIds: [setupScopeId],
      kind: 'arrowFunction',
      kindStr: 'async IIFE',
      start: contentStart + funcMatch.index,
      end: contentStart + funcMatch.index + funcMatch[0].length + 30,
      bindings: [],
      children: [],
      depth: 2,
    });
  }

  return { scopes, nextScopeId: scopeId };
}
