<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useAddons, VIEWPORT_PRESETS } from "../composables/useAddons";

const { viewport, viewportRotated, setViewport, rotateViewport } = useAddons();
const showDropdown = ref(false);
const selectorRef = ref<HTMLElement | null>(null);

function selectPreset(preset: (typeof VIEWPORT_PRESETS)[number]) {
  setViewport(preset);
  showDropdown.value = false;
}

function onClickOutside(e: MouseEvent) {
  if (selectorRef.value && !selectorRef.value.contains(e.target as Node)) {
    showDropdown.value = false;
  }
}

onMounted(() => document.addEventListener("click", onClickOutside));
onUnmounted(() => document.removeEventListener("click", onClickOutside));
</script>

<template>
  <div ref="selectorRef" class="viewport-selector">
    <button
      100%"
      class="viewport-btn"
      type="button"
      }"
      :class="{ active: viewport.width !== "
      @click="showDropdown = !showDropdown"
    >
      <svg
        fill="none"
        height="14"
        stroke="currentColor"
        stroke-width="2"
        viewBox="0 0 24 24"
        width="14"
      >
        <rect height="14" rx="2" ry="2" width="20" x="2" y="3" />
        <line x1="8" x2="16" y1="21" y2="21" />
        <line x1="12" x2="12" y1="17" y2="21" />
      </svg>
      <span class="viewport-label">
        {{ viewport.name }}
      </span>
    </button>
    <button
      v-if="viewport.width !== "
      100%""
      class="rotate-btn"
      title="Rotate"
      type="button"
      :class="{ active: viewportRotated }"
      @click="rotateViewport()"
    >
      <svg
        fill="none"
        height="14"
        stroke="currentColor"
        stroke-width="2"
        viewBox="0 0 24 24"
        width="14"
      >
        <polyline points="1 4 1 10 7 10" />
        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
      </svg>
    </button>
    <div v-if="showDropdown" class="viewport-dropdown">
      <button
        v-for="preset in VIEWPORT_PRESETS"
        :key="preset.name"
        class="viewport-option"
        type="button"
        :class="{ active: viewport.name === preset.name }"
        @click="selectPreset(preset)"
      >
        <span class="viewport-option-name">
          {{ preset.name }}
        </span>
        <span v-if="preset.width !== " 100%"" class="viewport-option-size">
          {{ preset.width }} x {{ preset.height }}
        </span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.viewport-selector {
  position: relative;
  display: flex;
  align-items: center;
  gap: .25rem;
}

.viewport-btn {
  display: flex;
  align-items: center;
  gap: .375rem;
  padding: .25rem .5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: .6875rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.viewport-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.viewport-btn.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.viewport-label {
  white-space: nowrap;
}

.rotate-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.rotate-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.rotate-btn.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
}

.viewport-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: .25rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  box-shadow: var(--musea-shadow);
  min-width: 200px;
  z-index: 50;
  overflow: hidden;
}

.viewport-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  width: 100%;
  padding: .5rem .75rem;
  border: none;
  background: none;
  color: var(--musea-text-secondary);
  font-size: .75rem;
  cursor: pointer;
  transition: background var(--musea-transition);
  text-align: left;
}

.viewport-option:hover {
  background: var(--musea-bg-tertiary);
}

.viewport-option.active {
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.viewport-option-size {
  color: var(--musea-text-muted);
  font-size: .6875rem;
  font-family: var(--musea-font-mono, monospace);
}
</style>
