<script setup lang="ts">
import { ref, watch } from "vue";
import type { DesignToken } from "../../api";

const props = defineProps<{
  isOpen: boolean;
  tokenPath: string;
  token?: DesignToken;
  dependents?: string[];
}>();

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();

const confirming = ref(false);

watch(
  () => props.isOpen,
  (open) => {
    if (open) confirming.value = false;
  },
);
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="modal-overlay" close")" @click.self="emit(">
        <div class="modal-content">
          <h3 class="modal-title">
            Delete Token
          </h3>
          <p class="delete-message">
            Are you sure you want to delete
            <code class="token-path">
              {{ tokenPath }}
            </code>
            ?
          </p>
          <div v-if="dependents && dependents.length > 0" class="dependents-warning">
            <svg
              fill="none"
              height="16"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
              width="16"
            >
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
              <line x1="12" x2="12" y1="9" y2="13" />
              <line x1="12" x2="12.01" y1="17" y2="17" />
            </svg>
            <div>
              <p class="warning-text">
                The following semantic tokens reference this token:
              </p>
              <ul class="dependents-list">
                <li v-for="dep in dependents" :key="dep" class="dependent-item">
                  <code>
                    {{ dep }}
                  </code>
                </li>
              </ul>
              <p class="warning-note">
                Deleting this token will leave these references unresolved.
              </p>
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn--secondary" close")" type="button" @click="emit(">
              Cancel
            </button>
            <button class="btn btn--danger" confirm")" type="button" @click="emit(">
              Delete
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--musea-overlay);
  backdrop-filter: blur(4px);
}

.modal-content {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg, 12px);
  width: 90%;
  max-width: 420px;
  padding: 1.5rem;
  box-shadow: var(--musea-shadow);
}

.modal-title {
  font-size: 1.125rem;
  font-weight: 700;
  margin-bottom: 1rem;
}

.delete-message {
  font-size: .875rem;
  color: var(--musea-text);
  margin-bottom: 1rem;
}

.token-path {
  font-family: var(--musea-font-mono);
  font-size: .8125rem;
  background: var(--musea-border);
  padding: .125rem .375rem;
  border-radius: var(--musea-radius-sm, 4px);
}

.dependents-warning {
  display: flex;
  gap: .75rem;
  padding: .75rem;
  background: #f59e0b1a;
  border: 1px solid #f59e0b4d;
  border-radius: var(--musea-radius-md);
  margin-bottom: 1rem;
  color: #fbbf24;
}

.dependents-warning svg {
  flex-shrink: 0;
  margin-top: .125rem;
}

.warning-text {
  font-size: .8125rem;
  margin-bottom: .375rem;
}

.dependents-list {
  list-style: none;
  padding: 0;
  margin-bottom: .375rem;
}

.dependent-item {
  font-size: .75rem;
  margin-bottom: .125rem;
}

.dependent-item code {
  font-family: var(--musea-font-mono);
}

.warning-note {
  font-size: .75rem;
  opacity: .8;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: .5rem;
}

.btn {
  padding: .5rem 1rem;
  border: none;
  border-radius: var(--musea-radius-md);
  font-size: .8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.btn--secondary {
  background: var(--musea-border);
  color: var(--musea-text);
}

.btn--secondary:hover {
  background: var(--musea-text-muted);
}

.btn--danger {
  background: #ef4444;
  color: #fff;
}

.btn--danger:hover {
  background: #dc2626;
}

.modal-enter-active, .modal-leave-active {
  transition: opacity .2s;
}

.modal-enter-active .modal-content, .modal-leave-active .modal-content {
  transition: transform .2s;
}

.modal-enter-from, .modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-content {
  transform: scale(.95);
}

.modal-leave-to .modal-content {
  transform: scale(.95);
}
</style>
