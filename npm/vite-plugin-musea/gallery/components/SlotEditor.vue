<script setup lang="ts">
import { ref, watch, computed, defineAsyncComponent } from "vue";

const MonacoEditor = defineAsyncComponent(() => import("./MonacoEditor.vue"));

const props = defineProps<{
  slots: Record<string, string>;
  availableSlots?: string[];
}>();

const emit = defineEmits<{
  (e: "update", slots: Record<string, string>): void;
}>();

const activeSlot = ref("default");
const localSlots = ref<Record<string, string>>({});

// Initialize local slots from props
watch(
  () => props.slots,
  (newSlots) => {
    localSlots.value = { ...newSlots };
  },
  { immediate: true, deep: true },
);

const slotNames = computed(() => {
  const names = new Set(["default"]);
  if (props.availableSlots) {
    for (const name of props.availableSlots) {
      names.add(name);
    }
  }
  for (const name of Object.keys(localSlots.value)) {
    names.add(name);
  }
  return Array.from(names);
});

const currentContent = computed({
  get: () => localSlots.value[activeSlot.value] || "",
  set: (value: string) => {
    localSlots.value[activeSlot.value] = value;
    emit("update", { ...localSlots.value });
  },
});

const selectSlot = (name: string) => {
  activeSlot.value = name;
};

const clearSlot = () => {
  localSlots.value[activeSlot.value] = "";
  emit("update", { ...localSlots.value });
};

const clearAllSlots = () => {
  localSlots.value = {};
  emit("update", {});
};
</script>

<template>
  <div class="slot-editor">
    <div class="slot-header">
      <div class="slot-tabs">
        <button
          v-for="name in slotNames"
          :key="name"
          "slot-tab--active":
          activeSlot="=="
          name
          slot-tab",
          type="button"
          {
          }]"
          :class="["
          @click="selectSlot(name)"
        >
          <span class="slot-tab-icon">
            #
          </span>
          {{ name }}
        </button>
      </div>
      <div class="slot-actions">
        <button class="slot-action" title="Clear current slot" type="button" @click="clearSlot">
          Clear
        </button>
        <button
          class="slot-action slot-action--danger"
          title="Clear all slots"
          type="button"
          @click="clearAllSlots"
        >
          Clear All
        </button>
      </div>
    </div>
    <div class="slot-content">
      <MonacoEditor v-model="currentContent" height="150px" language="html" />
    </div>
    <div class="slot-footer">
      <div class="slot-hint">
        <code>
          &lt;slot&gt;
        </code>
        = default,
        <code>
          &lt;slot name="foo"&gt;
        </code>
        = #foo
      </div>
    </div>
  </div>
</template>

<style scoped>
.slot-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--musea-bg-secondary);
  border-top: 1px solid var(--musea-border);
}

.slot-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: .5rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
}

.slot-tabs {
  display: flex;
  gap: .25rem;
}

.slot-tab {
  display: flex;
  align-items: center;
  gap: .25rem;
  padding: .375rem .625rem;
  background: none;
  border: 1px solid #0000;
  border-radius: 4px;
  font-size: .75rem;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all .15s;
}

.slot-tab:hover {
  background: var(--musea-bg-secondary);
  color: var(--musea-text-secondary);
}

.slot-tab--active {
  background: var(--musea-bg-secondary);
  border-color: var(--musea-accent);
  color: var(--musea-text);
}

.slot-tab-icon {
  font-family: var(--musea-font-mono);
  color: var(--musea-accent);
}

.slot-actions {
  display: flex;
  gap: .25rem;
}

.slot-action {
  padding: .25rem .5rem;
  background: none;
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: .6875rem;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all .15s;
}

.slot-action:hover {
  background: var(--musea-bg-secondary);
  color: var(--musea-text);
}

.slot-action--danger:hover {
  border-color: #f87171;
  color: #f87171;
}

.slot-content {
  flex: 1;
  overflow: hidden;
}

.slot-footer {
  padding: .375rem .75rem;
  background: var(--musea-bg-tertiary);
  border-top: 1px solid var(--musea-border);
}

.slot-hint {
  font-size: .6875rem;
  color: var(--musea-text-muted);
}

.slot-hint code {
  padding: .0625rem .25rem;
  background: var(--musea-bg-primary);
  border-radius: 2px;
  font-family: var(--musea-font-mono);
}
</style>
