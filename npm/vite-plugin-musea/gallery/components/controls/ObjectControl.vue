<script setup lang="ts">
import { ref, watch } from "vue";
import MonacoEditor from "../MonacoEditor.vue";

const props = defineProps<{
  label: string;
  description?: string;
  required?: boolean;
  modelValue?: unknown;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: unknown): void;
}>();

const jsonString = ref(JSON.stringify(props.modelValue ?? {}, null, 2));
const parseError = ref(false);

watch(
  () => props.modelValue,
  (val) => {
    const incoming = JSON.stringify(val ?? {}, null, 2);
    if (incoming !== jsonString.value) {
      jsonString.value = incoming;
      parseError.value = false;
    }
  },
);

function onEditorUpdate(value: string) {
  jsonString.value = value;
  try {
    const parsed = JSON.parse(value);
    parseError.value = false;
    emit("update:modelValue", parsed);
  } catch {
    parseError.value = true;
  }
}
</script>

<template>
  <div class="control">
    <label class="control-label">
      {{ label }}
      <span v-if="required" class="control-required">
        *
      </span>
      <span v-if="parseError" class="control-error-badge">
        Invalid JSON
      </span>
    </label>
    <div class="control-editor" :class="{ "has-error": parseError }">
      <MonacoEditor
        height="120px"
        language="json"
        :model-value="jsonString"
        @update:model-value="onEditorUpdate"
       />
    </div>
    <span v-if="description" class="control-desc">
      {{ description }}
    </span>
  </div>
</template>

<style scoped>
.control {
  display: flex;
  flex-direction: column;
  gap: .375rem;
}

.control-label {
  font-size: .75rem;
  font-weight: 600;
  color: var(--musea-text-secondary);
  display: flex;
  align-items: center;
  gap: .375rem;
}

.control-required {
  color: var(--musea-error);
}

.control-error-badge {
  font-size: .625rem;
  font-weight: 500;
  color: var(--musea-error);
  background: #f871711a;
  padding: .0625rem .375rem;
  border-radius: var(--musea-radius-sm);
}

.control-editor {
  border-radius: var(--musea-radius-sm);
  overflow: hidden;
  transition: box-shadow var(--musea-transition);
}

.control-editor.has-error {
  box-shadow: 0 0 0 1px var(--musea-error);
}

.control-desc {
  font-size: .6875rem;
  color: var(--musea-text-muted);
}
</style>
