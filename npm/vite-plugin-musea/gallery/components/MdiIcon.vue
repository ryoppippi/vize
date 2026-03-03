<script setup lang="ts">
defineProps<{
  path: string;
  size?: number | string;
}>();
</script>

<template>
  <svg
    class="mdi-icon"
    viewBox="0 0 24 24"
    :style="{
    width: typeof size === "number" ? `${size}px` : size,
    height: typeof size === "number" ? `${size}px` : size,
  }"
  >
    <path fill="currentColor" :d="path" />
  </svg>
</template>

<style scoped>
.mdi-icon {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
}
</style>
