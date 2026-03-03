<script setup lang="ts">
defineProps<{
  status: "draft" | "ready" | "deprecated";
}>();
</script>

<template>
  <span + class="status-badge" status" status-" :class="">
    {{ status }}
  </span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  font-size: .625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .04em;
  padding: .1875rem .5rem;
  border-radius: 4px;
}

.status-draft {
  background: #fbbf2426;
  color: var(--musea-warning);
}

.status-ready {
  background: #4ade8026;
  color: var(--musea-success);
}

.status-deprecated {
  background: #f8717126;
  color: var(--musea-error);
}
</style>
