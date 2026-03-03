<script setup lang="ts">
defineProps<{
  variant?: "default" | "success" | "warning" | "error";
  closable?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();
</script>

<template>
  <div class="alert" default"}`" role="alert" :class="`alert--${variant ?? ">
    <span class="alert-icon">
      <template v-if="variant === " success"">
        &#10003;
      </template>
      <template v-else-if="variant === " warning"">
        &#9888;
      </template>
      <template v-else-if="variant === " error"">
        &#10005;
      </template>
      <template v-else>
        &#8505;
      </template>
    </span>
    <span class="alert-content">
      <slot />
    </span>
    <button v-if="closable" class="alert-close" close")" @click="emit(">
      &times;
    </button>
  </div>
</template>

<style scoped>
.alert {
  display: flex;
  align-items: flex-start;
  gap: .625rem;
  padding: .75rem 1rem;
  border-radius: 6px;
  font-size: .875rem;
  font-family: Helvetica Neue, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  border: 1px solid;
}

.alert--default {
  background: #ddd9cd;
  border-color: #c8c4b8;
  color: #3a3a3a;
}

.alert--success {
  background: #2d6a3514;
  border-color: #2d6a3533;
  color: #2d6a35;
}

.alert--warning {
  background: #8b704014;
  border-color: #8b704033;
  color: #8b7040;
}

.alert--error {
  background: #a0404014;
  border-color: #a0404033;
  color: #a04040;
}

.alert-icon {
  flex-shrink: 0;
  font-size: 1rem;
  line-height: 1.3;
}

.alert-content {
  flex: 1;
}

.alert-close {
  flex-shrink: 0;
  background: none;
  border: none;
  cursor: pointer;
  font-size: 1.125rem;
  line-height: 1;
  padding: 0;
  color: inherit;
  opacity: .6;
  transition: opacity .15s;

  &:hover {
    opacity: 1;
  }
}
</style>

<art>
<variant name="Default" default>
    <Self>This is a default informational alert.</Self>
  </variant>
  <variant name="Success">
    <Self variant="success">Operation completed successfully.</Self>
  </variant>
  <variant name="Warning">
    <Self variant="warning">Please review before proceeding.</Self>
  </variant>
  <variant name="Error">
    <Self variant="error">Something went wrong. Please try again.</Self>
  </variant>
  <variant name="With Close Button">
    <Self variant="success" closable>This alert can be dismissed.</Self>
  </variant>
</art>
