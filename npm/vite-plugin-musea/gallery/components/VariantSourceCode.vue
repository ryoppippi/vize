<script setup lang="ts">
import { ref, computed } from "vue";
import hljs from "highlight.js/lib/core";
import xml from "highlight.js/lib/languages/xml";

hljs.registerLanguage("xml", xml);

const props = defineProps<{
  code: string;
}>();

const copied = ref(false);

const highlightedCode = computed(() => {
  try {
    return hljs.highlight(props.code, { language: "xml" }).value;
  } catch {
    return props.code;
  }
});

async function copyCode(code: string) {
  try {
    await navigator.clipboard.writeText(code);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    // fallback
  }
}
</script>

<template>
  <div class="source-code">
    <div class="source-header">
      <span class="source-label">
        Template
      </span>
      <button class="source-copy-btn" type="button" @click="copyCode(code)">
        <svg
          v-if="!copied"
          fill="none"
          height="12"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
          width="12"
        >
          <rect height="13" rx="2" ry="2" width="13" x="9" y="9" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
        <svg
          v-else
          fill="none"
          height="12"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
          width="12"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
        {{ copied ? "Copied!" : "Copy" }}
      </button>
    </div>
    <pre class="source-pre">
      <code class="source-code-text hljs" v-html="highlightedCode" />
    </pre>
  </div>
</template>

<style scoped>
.source-code {
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
  margin-top: .5rem;
}

.source-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: .375rem .75rem;
  background: var(--musea-bg-elevated);
  border-bottom: 1px solid var(--musea-border);
}

.source-label {
  font-size: .625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--musea-text-muted);
}

.source-copy-btn {
  display: flex;
  align-items: center;
  gap: .25rem;
  padding: .125rem .375rem;
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: .625rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.source-copy-btn:hover {
  color: var(--musea-text);
  border-color: var(--musea-text-muted);
}

.source-pre {
  margin: 0;
  padding: .75rem;
  background: var(--musea-bg-primary);
  overflow-x: auto;
  max-height: 200px;
}

.source-code-text {
  font-family: SF Mono, Fira Code, Consolas, monospace;
  font-size: .75rem;
  line-height: 1.6;
  color: var(--musea-text-secondary);
  white-space: pre;
}
</style>
