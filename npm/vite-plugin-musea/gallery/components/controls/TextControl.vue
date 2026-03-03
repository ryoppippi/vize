<script setup lang="ts">
const model = defineModel<string>({ default: "" });

defineProps<{
  label: string;
  description?: string;
  required?: boolean;
}>();
</script>

<template>
  <div class="control">
    <label class="control-label">
      {{ label }}
      <span v-if="required" class="control-required">
        *
      </span>
    </label>
    <input v-model="model" class="control-input" type="text">
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
}

.control-required {
  color: var(--musea-error);
}

.control-input {
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: .375rem .625rem;
  color: var(--musea-text);
  font-size: .8125rem;
  outline: none;
  transition: border-color var(--musea-transition);
}

.control-input:focus {
  border-color: var(--musea-accent);
}

.control-desc {
  font-size: .6875rem;
  color: var(--musea-text-muted);
}
</style>
