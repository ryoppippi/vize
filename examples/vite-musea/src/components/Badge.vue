<script setup lang="ts">
defineProps<{
  variant?: "default" | "success" | "warning" | "error" | "info";
  size?: "sm" | "md" | "lg";
}>();
</script>

<template>
  <span class="badge" :class="[`badge--${variant ?? "default"}`, `badge--${size ?? "md"}`]">
    <slot />
  </span>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  font-weight: 600;
  border-radius: 9999px;
  white-space: nowrap;
}

.badge--sm {
  padding: .125rem .5rem;
  font-size: .625rem;
}

.badge--md {
  padding: .25rem .625rem;
  font-size: .75rem;
}

.badge--lg {
  padding: .375rem .75rem;
  font-size: .875rem;
}

.badge--default {
  background: #d4d0c4;
  color: #3a3a3a;
}

.badge--success {
  background: #2d6a351f;
  color: #2d6a35;
}

.badge--warning {
  background: #8b70401f;
  color: #8b7040;
}

.badge--error {
  background: #a040401a;
  color: #a04040;
}

.badge--info {
  background: #4a6b8a1f;
  color: #4a6b8a;
}
</style>

<art>
<variant name="Default" default>
    <Self>Default</Self>
  </variant>
  <variant name="Variants">
    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center">
      <Self>Default</Self>
      <Self variant="success">Success</Self>
      <Self variant="warning">Warning</Self>
      <Self variant="error">Error</Self>
      <Self variant="info">Info</Self>
    </div>
  </variant>
  <variant name="Sizes">
    <div style="display: flex; gap: 0.5rem; align-items: center">
      <Self size="sm" variant="info">Small</Self>
      <Self size="md" variant="info">Medium</Self>
      <Self size="lg" variant="info">Large</Self>
    </div>
  </variant>
</art>
