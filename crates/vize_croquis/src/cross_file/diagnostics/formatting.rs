//! Rich formatting for cross-file diagnostics.
//!
//! Provides the [`to_markdown`](super::CrossFileDiagnostic::to_markdown) method
//! that renders a human-readable Markdown representation of each diagnostic.

use super::{CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity};

impl CrossFileDiagnostic {
    /// Generate rich markdown diagnostic message.
    pub fn to_markdown(&self) -> String {
        let mut out = String::with_capacity(512);

        // Severity badge
        let severity_badge = match self.severity {
            DiagnosticSeverity::Error => "🔴 **ERROR**",
            DiagnosticSeverity::Warning => "🟡 **WARNING**",
            DiagnosticSeverity::Info => "🔵 **INFO**",
            DiagnosticSeverity::Hint => "💡 **HINT**",
        };

        vize_carton::push_fmt!(out, "{severity_badge} `{}`\n\n", self.code());
        vize_carton::push_fmt!(out, "### {}\n\n", self.message);

        // Detailed explanation based on kind
        self.format_kind_details(&mut out);

        // Suggestion
        if let Some(suggestion) = &self.suggestion {
            vize_carton::push_fmt!(out, "\n**💡 Suggestion**: {suggestion}\n");
        }

        out
    }

    /// Write kind-specific detailed explanation into the output buffer.
    fn format_kind_details(&self, out: &mut String) {
        match &self.kind {
            CrossFileDiagnosticKind::ReactivityOutsideSetup {
                api_name,
                context_description,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{api_name}()` is called outside the setup context ({context_description}).\n\n",
                );
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("- 🔄 **State Pollution (CSRP)**: In SSR, module-level state is shared across requests, causing data leaks between users.\n");
                out.push_str("- 💾 **Memory Leak**: Reactive state created outside setup won't be cleaned up when the component unmounts.\n");
                out.push_str("- 🐛 **Unpredictable Behavior**: The reactivity system expects to track dependencies within component context.\n\n");
                out.push_str("**Correct usage**:\n\n");
                out.push_str("```vue\n");
                out.push_str("<script setup>\n");
                vize_carton::push_fmt!(
                    *out,
                    "const state = {api_name}(...) // ✅ Called in setup\n",
                );
                out.push_str("</script>\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::LifecycleOutsideSetup {
                hook_name,
                context_description,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{hook_name}` is called outside the setup context ({context_description}).\n\n",
                );
                out.push_str("**Why this fails**:\n\n");
                out.push_str(
                    "- Lifecycle hooks must be called **synchronously** during `setup()`.\n",
                );
                out.push_str("- They rely on the current component instance being set.\n");
                out.push_str("- Calling them elsewhere will throw an error or have no effect.\n\n");
            }
            CrossFileDiagnosticKind::WatcherOutsideSetup {
                api_name,
                context_description,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{}()` is called outside the setup context ({}).\n\n",
                    api_name,
                    context_description
                );
                out.push_str("**Why this causes memory leaks**:\n\n");
                out.push_str("- Watchers created in setup are **automatically stopped** when the component unmounts.\n");
                out.push_str(
                    "- Watchers created outside setup **run forever** until manually stopped.\n",
                );
                out.push_str("- Each component mount creates new watchers without cleanup → memory leak.\n\n");
                out.push_str("**If you need a global watcher**, store the stop handle:\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(*out, "const stop = {}(...)\n", api_name);
                out.push_str("// Later: stop()\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::SpreadBreaksReactivity {
                source_name,
                source_type,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: Spreading `{}` (a `{}`) creates a **non-reactive shallow copy**.\n\n",
                    source_name,
                    source_type
                );
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(
                    *out,
                    "const copy = {{ ...{} }} // ❌ copy is NOT reactive\n",
                    source_name
                );
                vize_carton::push_fmt!(
                    *out,
                    "{}.foo = 'bar' // copy.foo is still the old value\n",
                    source_name
                );
                out.push_str("```\n\n");
                out.push_str("**Fix**: Keep the reference, or use `toRefs()`:\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(
                    *out,
                    "const {{ foo, bar }} = toRefs({}) // ✅ foo, bar are refs\n",
                    source_name
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReassignmentBreaksReactivity {
                variable_name,
                original_type,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: Reassigning `{}` loses the original `{}` reference.\n\n",
                    variable_name,
                    original_type
                );
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(*out, "let {} = ref(0)\n", variable_name);
                vize_carton::push_fmt!(
                    *out,
                    "{} = ref(1) // ❌ Template still watches the OLD ref\n",
                    variable_name
                );
                out.push_str("```\n\n");
                out.push_str("**Fix**: Mutate the `.value` instead:\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(*out, "const {} = ref(0)\n", variable_name);
                vize_carton::push_fmt!(
                    *out,
                    "{}.value = 1 // ✅ Same ref, new value\n",
                    variable_name
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                source_name,
                destructured_keys,
                suggestion,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: Destructuring `{}` extracts plain values, losing reactivity.\n\n",
                    source_name
                );
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                let keys = destructured_keys
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                vize_carton::push_fmt!(
                    *out,
                    "const {{ {} }} = {} // ❌ {} are plain values\n",
                    keys,
                    source_name,
                    keys
                );
                out.push_str("```\n\n");
                vize_carton::push_fmt!(*out, "**Fix**: Use `{}()`:\n\n", suggestion);
                out.push_str("```ts\n");
                vize_carton::push_fmt!(
                    *out,
                    "const {{ {} }} = {}({}) // ✅ {} are refs\n",
                    keys,
                    suggestion,
                    source_name,
                    keys
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReactiveReferenceEscapes {
                variable_name,
                escaped_via,
                target_name,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: Reactive reference `{}` escapes its scope via {}.\n\n",
                    variable_name,
                    escaped_via
                );
                if let Some(target) = target_name {
                    vize_carton::push_fmt!(*out, "**Escaped to**: `{}`\n\n", target);
                }
                out.push_str("**Why this is implicit** (like Rust's move semantics):\n\n");
                out.push_str("```\n");
                out.push_str("┌─ setup() ─────────────────────────────┐\n");
                vize_carton::push_fmt!(
                    *out,
                    "│  const {} = reactive({{...}})          │\n",
                    variable_name
                );
                vize_carton::push_fmt!(
                    *out,
                    "│  someFunction({})  ←── reference escapes │\n",
                    variable_name
                );
                out.push_str("│          │                              │\n");
                out.push_str("│          ▼                              │\n");
                out.push_str("│  ┌─ someFunction() ─────────────────┐  │\n");
                vize_carton::push_fmt!(
                    *out,
                    "│  │  // {} is now accessible here    │  │\n",
                    variable_name
                );
                out.push_str("│  │  // mutations affect original     │  │\n");
                out.push_str("│  └────────────────────────────────────┘  │\n");
                out.push_str("└──────────────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Issues**:\n\n");
                out.push_str("- 🔍 **Hidden Data Flow**: Mutations happen \"at a distance\" - hard to trace.\n");
                out.push_str(
                    "- 🐛 **Unexpected Side Effects**: Function may modify your reactive state.\n",
                );
                out.push_str(
                    "- 📦 **Ownership Unclear**: Who \"owns\" this reactive object now?\n\n",
                );
                out.push_str("**Explicit alternatives**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Pass a readonly version\n");
                vize_carton::push_fmt!(*out, "someFunction(readonly({}))\n\n", variable_name);
                out.push_str("// Option 2: Pass a snapshot (non-reactive copy)\n");
                vize_carton::push_fmt!(*out, "someFunction({{ ...{} }})\n\n", variable_name);
                out.push_str("// Option 3: Pass specific values explicitly\n");
                vize_carton::push_fmt!(
                    *out,
                    "someFunction({}.id, {}.name)\n",
                    variable_name,
                    variable_name
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReactiveObjectMutatedAfterEscape {
                variable_name,
                mutation_site,
                escape_site,
            } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{}` is mutated after escaping its scope.\n\n",
                    variable_name
                );
                vize_carton::push_fmt!(*out, "- Escaped at offset: {}\n", escape_site);
                vize_carton::push_fmt!(*out, "- Mutated at offset: {}\n\n", mutation_site);
                out.push_str("**Timeline**:\n\n");
                out.push_str("```\n");
                vize_carton::push_fmt!(*out, "1. {} created in setup()\n", variable_name);
                vize_carton::push_fmt!(
                    *out,
                    "2. {} passed to external function (escape)\n",
                    variable_name
                );
                vize_carton::push_fmt!(
                    *out,
                    "3. {} mutated ← mutations may affect escaped reference!\n",
                    variable_name
                );
                out.push_str("```\n\n");
                out.push_str("**This is similar to Rust's borrow checker**:\n\n");
                out.push_str("- In Rust: `cannot mutate while borrowed`\n");
                out.push_str("- In Vue: mutations after escape create implicit coupling\n\n");
                out.push_str("**Consider**: Document the mutation contract or use `readonly()`.\n");
            }
            CrossFileDiagnosticKind::CircularReactiveDependency { cycle } => {
                out.push_str("**Problem**: Circular reactive dependency detected.\n\n");
                out.push_str("**Dependency Cycle**:\n\n");
                out.push_str("```\n");
                for (i, node) in cycle.iter().enumerate() {
                    if i == 0 {
                        vize_carton::push_fmt!(*out, "┌─→ {}\n", node);
                    } else if i == cycle.len() - 1 {
                        vize_carton::push_fmt!(*out, "│   ↓\n└── {} ───┘\n", node);
                    } else {
                        vize_carton::push_fmt!(*out, "│   ↓\n│   {}\n", node);
                    }
                }
                out.push_str("```\n\n");
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("- 💥 **Infinite Update Loops**: Changes propagate endlessly.\n");
                out.push_str("- 📚 **Stack Overflow Risk**: Deep recursion in reactive updates.\n");
                out.push_str("- 🐌 **Performance Degradation**: Wasted computation cycles.\n\n");
                out.push_str("**How to fix**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Use computed() to break the cycle\n");
                out.push_str("const derived = computed(() => {\n");
                out.push_str("  // Read without triggering write\n");
                out.push_str("  return transform(source.value)\n");
                out.push_str("})\n\n");
                out.push_str("// Option 2: Use watchEffect with explicit dependencies\n");
                out.push_str("watchEffect(() => {\n");
                out.push_str("  // One-way data flow only\n");
                out.push_str("})\n\n");
                out.push_str("// Option 3: Restructure to remove bidirectional dependency\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol { key, is_provide } => {
                let action = if *is_provide { "provide" } else { "inject" };
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{}('{}')` uses a string key instead of Symbol/InjectionKey.\n\n",
                    action,
                    key
                );
                out.push_str("**Why string keys are problematic**:\n\n");
                out.push_str("```\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  String Keys          │  Symbol/InjectionKey           │\n");
                out.push_str("├───────────────────────┼────────────────────────────────┤\n");
                out.push_str("│  ❌ Name collisions    │  ✅ Guaranteed uniqueness       │\n");
                out.push_str("│  ❌ No type safety     │  ✅ Full TypeScript inference   │\n");
                out.push_str("│  ❌ Refactoring breaks │  ✅ IDE rename support          │\n");
                out.push_str("│  ❌ Hard to trace      │  ✅ Go-to-definition works      │\n");
                out.push_str("└───────────────────────┴────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Name collision example**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// ComponentA.vue\n");
                vize_carton::push_fmt!(*out, "provide('{}', myData)\n\n", key);
                out.push_str("// LibraryX (unknown to you)\n");
                vize_carton::push_fmt!(*out, "provide('{}', otherData)  // 💥 Collision!\n", key);
                out.push_str("```\n\n");
                out.push_str("**Type-safe pattern with InjectionKey**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// injection-keys.ts\n");
                out.push_str("import type { InjectionKey, Ref } from 'vue'\n\n");
                out.push_str("export interface UserState {\n");
                out.push_str("  name: string\n");
                out.push_str("  id: number\n");
                out.push_str("}\n\n");
                out.push_str(
                    "export const UserKey: InjectionKey<Ref<UserState>> = Symbol('user')\n\n",
                );
                out.push_str("// Provider.vue\n");
                out.push_str("import { UserKey } from './injection-keys'\n");
                out.push_str("provide(UserKey, userData)  // ✅ Type-checked\n\n");
                out.push_str("// Consumer.vue\n");
                out.push_str("import { UserKey } from './injection-keys'\n");
                out.push_str(
                    "const user = inject(UserKey)  // ✅ Type: Ref<UserState> | undefined\n",
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::WatchMutationCanBeComputed {
                watch_source,
                mutated_target,
                suggested_computed,
            } => {
                out.push_str("**Problem**: This `watch` callback only mutates a reactive value based on its source.\n\n");
                out.push_str("**Current code** (imperative, harder to trace):\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(*out, "watch({}, (newVal) => {{\n", watch_source);
                vize_carton::push_fmt!(*out, "  {}.value = transform(newVal)\n", mutated_target);
                out.push_str("})\n");
                out.push_str("```\n\n");
                out.push_str("**Why `computed` is better**:\n\n");
                out.push_str("```\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  watch + mutation       │  computed                     │\n");
                out.push_str("├─────────────────────────┼───────────────────────────────┤\n");
                out.push_str("│  ❌ Imperative flow      │  ✅ Declarative transformation │\n");
                out.push_str("│  ❌ Two variables        │  ✅ Single derived value       │\n");
                out.push_str("│  ❌ Manual sync needed   │  ✅ Auto-cached and reactive   │\n");
                out.push_str("│  ❌ Side effects possible│  ✅ Pure function guarantee    │\n");
                out.push_str("└─────────────────────────┴───────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Refactored code** (declarative, easier to reason about):\n\n");
                out.push_str("```ts\n");
                vize_carton::push_fmt!(*out, "{}\n", suggested_computed);
                out.push_str("```\n\n");
                out.push_str("**Note**: Use `watch` only when you need **side effects** (API calls, logging, etc.).\n");
            }
            CrossFileDiagnosticKind::DomAccessWithoutNextTick { api, context } => {
                vize_carton::push_fmt!(
                    *out,
                    "**Problem**: `{}` is accessed in `{}` without `nextTick()`.\n\n",
                    api,
                    context
                );
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("```\n");
                out.push_str("Timeline of Vue component lifecycle:\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  1. setup() runs        → DOM does NOT exist yet        │\n");
                out.push_str("│  2. Template renders    → Virtual DOM created           │\n");
                out.push_str("│  3. onMounted() fires   → DOM exists now                │\n");
                out.push_str("│  4. nextTick() resolves → DOM is fully updated          │\n");
                out.push_str("└─────────────────────────────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**SSR considerations**:\n\n");
                out.push_str("- On the server, `document` and `window` don't exist at all.\n");
                out.push_str(
                    "- Accessing them throws `ReferenceError: document is not defined`.\n\n",
                );
                out.push_str("**Safe patterns**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Use inside onMounted\n");
                out.push_str("onMounted(() => {\n");
                vize_carton::push_fmt!(*out, "  {}  // ✅ Safe - DOM exists\n", api);
                out.push_str("})\n\n");
                out.push_str("// Option 2: Use nextTick after state change\n");
                out.push_str("await nextTick()\n");
                vize_carton::push_fmt!(*out, "{}  // ✅ Safe - DOM updated\n", api);
                out.push('\n');
                out.push_str("// Option 3: Guard for SSR\n");
                out.push_str("if (typeof document !== 'undefined') {\n");
                vize_carton::push_fmt!(*out, "  {}  // ✅ Safe - browser only\n", api);
                out.push_str("}\n");
                out.push_str("```\n");
            }
            _ => {
                // Default: just show the message
            }
        }
    }
}
