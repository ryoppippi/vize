<script setup lang="ts">
import {
  ref,
  watch,
  computed,
  onMounted,
  onUnmounted,
  nextTick,
  getCurrentInstance,
  inject,
  type ComputedRef,
} from "vue";
import MonacoEditor from "../../shared/MonacoEditor.vue";
import type { Diagnostic } from "../../shared/MonacoEditor.vue";
import type {
  WasmModule,
  CroquisResult,
  BindingDisplay,
  BindingSource,
  ScopeDisplay,
} from "../../wasm/index";
import { ANALYSIS_PRESET } from "../../shared/presets/croquis";
import { mdiCodeTags, mdiChartTimelineVariant, mdiCheck, mdiCloseCircle, mdiAlert } from "@mdi/js";

const props = defineProps<{
  compiler: WasmModule | null;
}>();
const _injectedTheme = inject<ComputedRef<"dark" | "light">>("theme");
const theme = computed<"dark" | "light">(() => _injectedTheme?.value ?? "light");

const source = ref(ANALYSIS_PRESET);
const analysisResult = ref<CroquisResult | null>(null);
const error = ref<string | null>(null);
const activeTab = ref<"vir" | "stats" | "bindings" | "scopes" | "diagnostics">("vir");
const showScopeVisualization = ref(true);
const editorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);

// Computed values for display (declared early so scopeDecorations can reference scopes)
const summary = computed(() => analysisResult.value?.croquis);
const scopes = computed(() => summary.value?.scopes || []);

// Convert scopes to Monaco editor decorations (exclude module scope and global scopes with no position)
const scopeDecorations = computed(() => {
  if (!scopes.value) return [];
  return scopes.value
    .filter((scope) => {
      // Don't visualize module scope (covers entire file)
      if (scope.kind === "mod") return false;
      // Don't visualize global scopes (start/end = 0)
      if (scope.start === 0 && scope.end === 0) return false;
      return true;
    })
    .map((scope) => ({
      start: scope.start,
      end: scope.end,
      kind: scope.kind,
      kindStr: scope.kindStr,
    }));
});

// Scopes to pass to MonacoEditor (separate computed for reliable reactivity)
const editorScopes = computed(() => (showScopeVisualization.value ? scopeDecorations.value : []));

// Directly push scope decorations to MonacoEditor when they change
// (handles showScopeVisualization toggle)
watch(editorScopes, async (scopes) => {
  await nextTick();
  editorRef.value?.applyScopeDecorations(scopes);
});

const analysisTime = ref<number | null>(null);

// Workaround for Vize compiler prop reactivity issue:
// instance.props.compiler stays null even after parent updates,
// but vnode.props.compiler has the correct value.
const _instance = getCurrentInstance();
function getCompiler(): WasmModule | null {
  return (
    props.compiler ??
    ((_instance?.vnode?.props as Record<string, unknown>)?.compiler as WasmModule | null) ??
    null
  );
}

// Perform analysis
async function analyze() {
  const compiler = getCompiler();
  if (!compiler) {
    error.value = "Compiler not loaded";
    return;
  }

  error.value = null;
  const startTime = performance.now();

  try {
    const result = compiler.analyzeSfc(source.value, {
      filename: "Component.vue",
    });
    analysisResult.value = result;
    analysisTime.value = performance.now() - startTime;

    // Directly apply scope decorations after analysis
    // (bypasses Vize compiler prop reactivity issue)
    await nextTick();
    if (editorRef.value && showScopeVisualization.value) {
      editorRef.value.applyScopeDecorations(scopeDecorations.value);
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    analysisResult.value = null;
  }
}

// Watch for source changes and re-analyze
let analyzeTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  source,
  () => {
    if (analyzeTimer) clearTimeout(analyzeTimer);
    analyzeTimer = setTimeout(analyze, 300);
  },
  { immediate: false },
);

// Watch for compiler changes
watch(
  () => props.compiler,
  () => {
    if (getCompiler()) {
      analyze();
    }
  },
);

// Analyze on mount (with polling fallback for Vize compiler prop reactivity issue)
let compilerPollTimer: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  if (getCompiler()) {
    analyze();
  } else {
    compilerPollTimer = setInterval(() => {
      if (getCompiler()) {
        if (compilerPollTimer) clearInterval(compilerPollTimer);
        compilerPollTimer = null;
        analyze();
      }
    }, 200);
  }
});
onUnmounted(() => {
  if (compilerPollTimer) clearInterval(compilerPollTimer);
});

// Computed values for display
const bindings = computed(() => summary.value?.bindings || []);
const macros = computed(() => summary.value?.macros || []);
const css = computed(() => summary.value?.css);
const typeExports = computed(() => summary.value?.typeExports || []);
const invalidExports = computed(() => summary.value?.invalidExports || []);
const diagnostics = computed(() => analysisResult.value?.diagnostics || []);
const stats = computed(() => summary.value?.stats);

// Convert character offset to line/column (1-based for Monaco)
function offsetToLineColumn(content: string, offset: number): { line: number; column: number } {
  const beforeOffset = content.substring(0, offset);
  const lines = beforeOffset.split("\n");
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

// Monaco-compatible diagnostics (converted from offset-based to line/column)
const monacoDiagnostics = computed<Diagnostic[]>(() => {
  return diagnostics.value.map((d) => {
    const start = offsetToLineColumn(source.value, d.start);
    const end = offsetToLineColumn(source.value, d.end);
    return {
      message: d.message,
      startLine: start.line,
      startColumn: start.column,
      endLine: end.line,
      endColumn: end.column,
      severity: d.severity === "hint" ? "info" : (d.severity as "error" | "warning" | "info"),
    };
  });
});

// Group bindings by source
const bindingsBySource = computed(() => {
  const groups: Record<string, BindingDisplay[]> = {};
  for (const binding of bindings.value) {
    const source = binding.source || "unknown";
    if (!groups[source]) groups[source] = [];
    groups[source].push(binding);
  }
  return groups;
});

// VIR (Vize Intermediate Representation) text
const virText = computed(() => analysisResult.value?.vir || "");

// Token types for VIR syntax highlighting
type VirTokenType =
  | "border"
  | "section"
  | "section-name"
  | "macro"
  | "type"
  | "binding"
  | "identifier"
  | "tag"
  | "source"
  | "arrow"
  | "number"
  | "diagnostic"
  | "keyword"
  | "colon"
  | "bracket"
  | "plain";

interface VirToken {
  type: VirTokenType;
  text: string;
}

interface VirLine {
  tokens: VirToken[];
  index: number;
  lineType: string;
}

// Tokenize a VIR line for fine-grained syntax highlighting
function tokenizeVirLine(line: string): VirToken[] {
  const tokens: VirToken[] = [];
  let remaining = line;

  while (remaining.length > 0) {
    let matched = false;

    // Border characters: ╭╰│├└─┌┐╮╯┤┬┴┼
    const borderMatch = remaining.match(/^[╭╰│├└─┌┐╮╯┤┬┴┼]+/);
    if (borderMatch) {
      tokens.push({ type: "border", text: borderMatch[0] });
      remaining = remaining.slice(borderMatch[0].length);
      matched = true;
      continue;
    }

    // Section marker ■
    if (remaining.startsWith("■")) {
      tokens.push({ type: "section", text: "■" });
      remaining = remaining.slice(1);
      matched = true;
      continue;
    }

    // Section names (all caps words)
    const sectionNameMatch = remaining.match(
      /^(MACROS|BINDINGS|SCOPES|PROPS|EMITS|CSS|DIAGNOSTICS|STATS|SUMMARY)/,
    );
    if (sectionNameMatch) {
      tokens.push({ type: "section-name", text: sectionNameMatch[0] });
      remaining = remaining.slice(sectionNameMatch[0].length);
      matched = true;
      continue;
    }

    // Macro names @defineProps, @defineEmits, etc.
    const macroMatch = remaining.match(/^@\w+/);
    if (macroMatch) {
      tokens.push({ type: "macro", text: macroMatch[0] });
      remaining = remaining.slice(macroMatch[0].length);
      matched = true;
      continue;
    }

    // Type annotations <...>
    const typeMatch = remaining.match(/^<[^>]+>/);
    if (typeMatch) {
      tokens.push({ type: "type", text: typeMatch[0] });
      remaining = remaining.slice(typeMatch[0].length);
      matched = true;
      continue;
    }

    // Binding marker ▸
    if (remaining.startsWith("▸")) {
      tokens.push({ type: "binding", text: "▸" });
      remaining = remaining.slice(1);
      matched = true;
      continue;
    }

    // Arrow →
    if (remaining.startsWith("→")) {
      tokens.push({ type: "arrow", text: "→" });
      remaining = remaining.slice(1);
      matched = true;
      continue;
    }

    // Diagnostic icons
    const diagMatch = remaining.match(/^[✗⚠ℹ✓]/);
    if (diagMatch) {
      tokens.push({ type: "diagnostic", text: diagMatch[0] });
      remaining = remaining.slice(diagMatch[0].length);
      matched = true;
      continue;
    }

    // Tags in brackets [SetupRef], [Module], etc.
    const tagMatch = remaining.match(/^\[[A-Za-z][A-Za-z0-9_]*\]/);
    if (tagMatch) {
      tokens.push({ type: "tag", text: tagMatch[0] });
      remaining = remaining.slice(tagMatch[0].length);
      matched = true;
      continue;
    }

    // Keywords like type:, args:, scoped:, etc.
    const keywordMatch = remaining.match(
      /^(type|args|scoped|selectors|v-bind|start|end|depth|parent|bindings|children):/,
    );
    if (keywordMatch) {
      tokens.push({ type: "keyword", text: keywordMatch[1] });
      tokens.push({ type: "colon", text: ":" });
      remaining = remaining.slice(keywordMatch[0].length);
      matched = true;
      continue;
    }

    // Source types (ref, computed, props, etc.) - after keywords
    const sourceMatch = remaining.match(
      /^\b(ref|computed|reactive|props|emits|local|import|function|class|unknown)\b/,
    );
    if (sourceMatch) {
      tokens.push({ type: "source", text: sourceMatch[0] });
      remaining = remaining.slice(sourceMatch[0].length);
      matched = true;
      continue;
    }

    // Numbers including ranges like [0:100]
    const numberMatch = remaining.match(/^\d+/);
    if (numberMatch) {
      tokens.push({ type: "number", text: numberMatch[0] });
      remaining = remaining.slice(numberMatch[0].length);
      matched = true;
      continue;
    }

    // Brackets and braces
    const bracketMatch = remaining.match(/^[[\]{}()]/);
    if (bracketMatch) {
      tokens.push({ type: "bracket", text: bracketMatch[0] });
      remaining = remaining.slice(bracketMatch[0].length);
      matched = true;
      continue;
    }

    // Colons (standalone)
    if (remaining.startsWith(":")) {
      tokens.push({ type: "colon", text: ":" });
      remaining = remaining.slice(1);
      matched = true;
      continue;
    }

    // Identifiers (variable names, etc.)
    const identMatch = remaining.match(/^[a-zA-Z_][a-zA-Z0-9_]*/);
    if (identMatch) {
      tokens.push({ type: "identifier", text: identMatch[0] });
      remaining = remaining.slice(identMatch[0].length);
      matched = true;
      continue;
    }

    // Whitespace
    const wsMatch = remaining.match(/^\s+/);
    if (wsMatch) {
      tokens.push({ type: "plain", text: wsMatch[0] });
      remaining = remaining.slice(wsMatch[0].length);
      matched = true;
      continue;
    }

    // Any other character
    if (!matched) {
      tokens.push({ type: "plain", text: remaining[0] });
      remaining = remaining.slice(1);
    }
  }

  return tokens;
}

// Determine line type for overall styling
function getVirLineType(line: string): string {
  if (line.startsWith("╭") || line.startsWith("│") || line.startsWith("╰")) return "header";
  if (line.includes("■")) return "section";
  if (line.includes("@define") || line.includes("┌─ @")) return "macro";
  if (line.includes("▸")) return "binding";
  if (line.includes("├─") || line.includes("└─")) return "tree";
  if (line.includes("✗") || line.includes("⚠")) return "diagnostic";
  return "plain";
}

// Parse VIR text into tokenized lines
const virLines = computed((): VirLine[] => {
  if (!virText.value) return [];
  const lines = virText.value.split("\n");
  // Remove trailing empty line if present
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }
  return lines.map((line, index) => ({
    tokens: tokenizeVirLine(line),
    index,
    lineType: getVirLineType(line),
  }));
});

// Source labels
function getSourceLabel(source: BindingSource | string): string {
  const labels: Record<string, string> = {
    props: "Props",
    emits: "Emits",
    model: "Models",
    slots: "Slots",
    ref: "Refs",
    reactive: "Reactive",
    computed: "Computed",
    import: "Imports",
    local: "Local",
    function: "Functions",
    class: "Classes",
    templateRef: "Template Refs",
    unknown: "Other",
  };
  return labels[source] || source;
}

// Source colors
function getSourceClass(source: BindingSource | string): string {
  const classes: Record<string, string> = {
    props: "src-props",
    emits: "src-emits",
    model: "src-model",
    slots: "src-slots",
    ref: "src-ref",
    reactive: "src-reactive",
    computed: "src-computed",
    import: "src-import",
    local: "src-local",
    function: "src-function",
    class: "src-class",
  };
  return classes[source] || "src-default";
}

// Scope kind colors
function getScopeColorClass(kind: string): string {
  // Direct mapping for exact matches
  const classes: Record<string, string> = {
    // Module scope
    mod: "scope-module",
    Mod: "scope-module",
    module: "scope-module",
    Module: "scope-module",
    // Plain (non-script-setup)
    plain: "scope-non-script-setup",
    Plain: "scope-non-script-setup",
    nonScriptSetup: "scope-non-script-setup",
    NonScriptSetup: "scope-non-script-setup",
    // Script setup
    setup: "scope-script-setup",
    Setup: "scope-script-setup",
    scriptSetup: "scope-script-setup",
    ScriptSetup: "scope-script-setup",
    // Function scopes
    function: "scope-function",
    Function: "scope-function",
    arrowFunction: "scope-function",
    ArrowFunction: "scope-function",
    block: "scope-block",
    Block: "scope-block",
    Callback: "scope-callback",
    // Template scopes
    vFor: "scope-vfor",
    VFor: "scope-vfor",
    vSlot: "scope-vslot",
    VSlot: "scope-vslot",
    EventHandler: "scope-event-handler",
    // External modules
    extern: "scope-external-module",
    extmod: "scope-external-module",
    ExternalModule: "scope-external-module",
    // SSR scopes
    universal: "scope-universal",
    Universal: "scope-universal",
    JsGlobal: "scope-js-global-universal",
    client: "scope-client-only",
    Client: "scope-client-only",
    clientOnly: "scope-client-only",
    ClientOnly: "scope-client-only",
    server: "scope-js-global-node",
    Server: "scope-js-global-node",
    // JS Global scopes (runtime-specific)
    jsGlobalUniversal: "scope-js-global-universal",
    JsGlobalUniversal: "scope-js-global-universal",
    jsGlobalBrowser: "scope-js-global-browser",
    JsGlobalBrowser: "scope-js-global-browser",
    jsGlobalNode: "scope-js-global-node",
    JsGlobalNode: "scope-js-global-node",
    jsGlobalDeno: "scope-js-global-deno",
    JsGlobalDeno: "scope-js-global-deno",
    jsGlobalBun: "scope-js-global-bun",
    JsGlobalBun: "scope-js-global-bun",
    // Vue global
    vue: "scope-vue-global",
    Vue: "scope-vue-global",
    vueGlobal: "scope-vue-global",
    VueGlobal: "scope-vue-global",
  };

  // Check for exact match
  if (classes[kind]) {
    return classes[kind];
  }

  // Check for partial matches (e.g., "ClientOnly (onMounted)" should match 'scope-client-only')
  if (kind.startsWith("ClientOnly")) return "scope-client-only";
  if (kind.startsWith("Universal")) return "scope-universal";
  if (kind.startsWith("ServerOnly")) return "scope-js-global-node";
  if (kind.startsWith("Function")) return "scope-function";
  if (kind.startsWith("Arrow")) return "scope-function";
  if (kind.startsWith("ExtMod")) return "scope-external-module";
  if (kind.startsWith("v-for")) return "scope-vfor";
  if (kind.startsWith("v-slot")) return "scope-vslot";
  if (kind.startsWith("@")) return "scope-event-handler"; // Event handlers like @click
  if (kind.includes("computed")) return "scope-function";
  if (kind.includes("watch")) return "scope-function";

  return "scope-default";
}
</script>

<template>
  <div class="croquis-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <svg class="icon" viewBox="0 0 24 24"><path :d="mdiCodeTags" fill="currentColor" /></svg>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <label class="toggle-label">
            <input type="checkbox" v-model="showScopeVisualization" />
            <span>Visualize Scopes</span>
          </label>
          <button @click="source = ANALYSIS_PRESET" class="btn-ghost">Reset</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor
          ref="editorRef"
          v-model="source"
          language="vue"
          :scopes="editorScopes"
          :diagnostics="monacoDiagnostics"
          :theme="theme"
        />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <svg class="icon" viewBox="0 0 24 24">
            <path :d="mdiChartTimelineVariant" fill="currentColor" />
          </svg>
          <h2>Semantic Analysis</h2>
          <span v-if="analysisTime !== null" class="perf-badge">
            {{ analysisTime.toFixed(2) }}ms
          </span>
        </div>
        <div class="tabs">
          <button :class="['tab', { active: activeTab === 'vir' }]" @click="activeTab = 'vir'">
            VIR
          </button>
          <button :class="['tab', { active: activeTab === 'stats' }]" @click="activeTab = 'stats'">
            Stats
          </button>
          <button
            :class="['tab', { active: activeTab === 'bindings' }]"
            @click="activeTab = 'bindings'"
          >
            Bindings
          </button>
          <button
            :class="['tab', { active: activeTab === 'scopes' }]"
            @click="activeTab = 'scopes'"
          >
            Scopes
          </button>
          <button
            :class="['tab', { active: activeTab === 'diagnostics' }]"
            @click="activeTab = 'diagnostics'"
          >
            Diagnostics
            <span v-if="diagnostics.length > 0" class="tab-badge">{{ diagnostics.length }}</span>
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Analysis Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="analysisResult">
          <!-- VIR Tab (Primary) -->
          <div v-if="activeTab === 'vir'" class="vir-output">
            <div class="vir-header-bar">
              <span class="vir-title">VIR — Vize Intermediate Representation</span>
              <span class="vir-line-count">{{ virLines.length }} lines</span>
            </div>
            <div class="vir-notice">
              VIR is a human-readable display format for debugging purposes only. It is not portable
              and should not be parsed or used as a stable interface.
            </div>
            <div class="vir-content">
              <div class="vir-line-numbers">
                <span v-for="(_, i) in virLines" :key="i" class="vir-ln">{{ i + 1 }}</span>
              </div>
              <div class="vir-code">
                <div
                  v-for="line in virLines"
                  :key="line.index"
                  :class="['vir-line', `vir-line-${line.lineType}`]"
                >
                  <template v-if="line.tokens.length > 0"
                    ><span
                      v-for="(token, ti) in line.tokens"
                      :key="ti"
                      :class="['vir-token', `vir-${token.type}`]"
                      >{{ token.text }}</span
                    ></template
                  ><template v-else><span>&#160;</span></template>
                </div>
              </div>
            </div>
          </div>

          <!-- Stats Tab -->
          <div v-else-if="activeTab === 'stats'" class="stats-output">
            <div class="stats-grid">
              <div class="stat-box">
                <div class="stat-number">{{ stats?.binding_count || 0 }}</div>
                <div class="stat-label">Bindings</div>
              </div>
              <div class="stat-box">
                <div class="stat-number">{{ stats?.macro_count || 0 }}</div>
                <div class="stat-label">Macros</div>
              </div>
              <div class="stat-box">
                <div class="stat-number">{{ stats?.scope_count || 0 }}</div>
                <div class="stat-label">Scopes</div>
              </div>
              <div class="stat-box">
                <div class="stat-number">{{ css?.v_bind_count || 0 }}</div>
                <div class="stat-label">v-bind()</div>
              </div>
            </div>

            <div class="section">
              <h3 class="section-title">Compiler Macros</h3>
              <div v-if="macros.length === 0" class="empty-state">No macros detected</div>
              <div v-else class="macro-list">
                <div
                  v-for="macro in macros"
                  :key="`${macro.name}-${macro.start}`"
                  class="macro-item"
                >
                  <span class="macro-name">{{ macro.name }}</span>
                  <code v-if="macro.type_args" class="macro-type">{{ macro.type_args }}</code>
                  <span v-if="macro.binding" class="macro-binding">→ {{ macro.binding }}</span>
                </div>
              </div>
            </div>

            <div class="section" v-if="css">
              <h3 class="section-title">CSS Analysis</h3>
              <div class="css-info">
                <span class="css-stat">{{ css.selector_count }} selectors</span>
                <span v-if="css.is_scoped" class="css-badge scoped">scoped</span>
                <span v-if="css.v_bind_count > 0" class="css-badge vbind"
                  >{{ css.v_bind_count }} v-bind</span
                >
              </div>
            </div>

            <div class="section" v-if="typeExports.length > 0">
              <h3 class="section-title">Type Exports <span class="badge hoisted">hoisted</span></h3>
              <div class="export-list">
                <div
                  v-for="te in typeExports"
                  :key="`${te.name}-${te.start}`"
                  class="export-item valid"
                >
                  <span class="export-kind">{{ te.kind }}</span>
                  <code class="export-name">{{ te.name }}</code>
                  <span class="export-badge hoisted">hoisted to module</span>
                </div>
              </div>
            </div>

            <div class="section" v-if="invalidExports.length > 0">
              <h3 class="section-title">Invalid Exports <span class="badge error">error</span></h3>
              <div class="export-list">
                <div
                  v-for="ie in invalidExports"
                  :key="`${ie.name}-${ie.start}`"
                  class="export-item invalid"
                >
                  <span class="export-kind">{{ ie.kind }}</span>
                  <code class="export-name">{{ ie.name }}</code>
                  <span class="export-badge error">not allowed in script setup</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Bindings Tab -->
          <div v-else-if="activeTab === 'bindings'" class="bindings-output">
            <div v-if="bindings.length === 0" class="empty-state">No bindings detected</div>

            <template v-else>
              <div v-for="(group, source) in bindingsBySource" :key="source" class="source-group">
                <div class="source-header">
                  <span :class="['source-indicator', getSourceClass(String(source))]"></span>
                  <span class="source-name">{{ getSourceLabel(String(source)) }}</span>
                  <span class="source-count">{{ group.length }}</span>
                </div>
                <div class="binding-grid">
                  <div v-for="binding in group" :key="binding.name" class="binding-item">
                    <div class="binding-main">
                      <code class="binding-name">{{ binding.name }}</code>
                      <span
                        v-if="binding.metadata?.needsValue"
                        class="needs-value"
                        title="Needs .value"
                        >.value</span
                      >
                    </div>
                    <div class="binding-meta">
                      <span class="binding-kind">{{ binding.kind }}</span>
                      <span v-if="binding.typeAnnotation" class="binding-type"
                        >: {{ binding.typeAnnotation }}</span
                      >
                    </div>
                    <div class="binding-flags">
                      <span
                        :class="['flag', binding.bindable ? 'active' : 'inactive']"
                        title="Can be referenced from template"
                        >bindable</span
                      >
                      <span
                        :class="['flag', binding.usedInTemplate ? 'active' : 'inactive']"
                        title="Actually used in template"
                        >in-template</span
                      >
                      <span :class="['flag', binding.isMutated ? 'active' : 'inactive']"
                        >mutated</span
                      >
                      <span
                        v-if="binding.fromScriptSetup"
                        class="flag setup"
                        title="From script setup"
                        >setup</span
                      >
                      <span class="refs">{{ binding.referenceCount }} refs</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- Scopes Tab -->
          <div v-else-if="activeTab === 'scopes'" class="scopes-output">
            <div v-if="scopes.length === 0" class="empty-state">No scopes detected</div>

            <div v-else class="scope-tree">
              <div
                v-for="scope in scopes"
                :key="scope.id"
                :class="['scope-node', getScopeColorClass(scope.kindStr || scope.kind)]"
                :style="{ marginLeft: `${(scope.depth || 0) * 20}px` }"
              >
                <div class="scope-header">
                  <span
                    :class="['scope-indicator', getScopeColorClass(scope.kindStr || scope.kind)]"
                  ></span>
                  <span class="scope-kind">{{ scope.kindStr || scope.kind }}</span>
                  <span class="scope-range">[{{ scope.start }}:{{ scope.end }}]</span>
                </div>
                <div v-if="scope.bindings.length > 0" class="scope-bindings">
                  <span v-for="name in scope.bindings" :key="name" class="scope-binding">{{
                    name
                  }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Diagnostics Tab -->
          <div v-else-if="activeTab === 'diagnostics'" class="diagnostics-output">
            <div v-if="diagnostics.length === 0" class="success-state">
              <svg class="success-icon" viewBox="0 0 24 24">
                <path :d="mdiCheck" fill="currentColor" />
              </svg>
              <span>No issues found</span>
            </div>

            <div v-else class="diagnostic-list">
              <div
                v-for="(diag, i) in diagnostics"
                :key="i"
                :class="['diagnostic-item', `severity-${diag.severity}`]"
              >
                <div class="diagnostic-header">
                  <svg class="severity-icon" viewBox="0 0 24 24">
                    <path
                      :d="diag.severity === 'error' ? mdiCloseCircle : mdiAlert"
                      fill="currentColor"
                    />
                  </svg>
                  <span class="diagnostic-message">{{ diag.message }}</span>
                </div>
                <div class="diagnostic-location">
                  <span class="location-range">{{ diag.start }}:{{ diag.end }}</span>
                  <span v-if="diag.code" class="diagnostic-code">[{{ diag.code }}]</span>
                </div>
              </div>
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Analyzing...</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped src="./CroquisPlayground.css"></style>
