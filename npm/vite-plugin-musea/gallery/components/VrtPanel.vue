<script setup lang="ts">
import { ref, computed } from "vue";
import { mdiPlay, mdiLoading, mdiImageOutline } from "@mdi/js";
import { runVrt } from "../api";
import MdiIcon from "./MdiIcon.vue";

const props = defineProps<{
  artPath: string;
  defaultVariantName?: string;
}>();

interface VrtResult {
  artPath: string;
  variantName: string;
  viewport: string;
  passed: boolean;
  isNew?: boolean;
  diffPercentage?: number;
  error?: string;
}

interface VrtSummary {
  total: number;
  passed: number;
  failed: number;
  new: number;
}

const isRunning = ref(false);
const hasRun = ref(false);
const results = ref<VrtResult[]>([]);
const summary = ref<VrtSummary | null>(null);
const error = ref<string | null>(null);
const updateSnapshots = ref(false);

const groupedResults = computed(() => {
  const groups: Record<string, VrtResult[]> = {};
  for (const r of results.value) {
    const key = r.variantName;
    if (!groups[key]) groups[key] = [];
    groups[key].push(r);
  }
  return groups;
});

async function runTest() {
  isRunning.value = true;
  error.value = null;

  try {
    const data = await runVrt(props.artPath, updateSnapshots.value);
    results.value = data.results;
    summary.value = data.summary;
    hasRun.value = true;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    isRunning.value = false;
  }
}

function getStatusIcon(result: VrtResult): string {
  if (result.error) return "error";
  if (result.isNew) return "new";
  if (result.passed) return "pass";
  return "fail";
}

function getStatusColor(result: VrtResult): string {
  if (result.error) return "#f87171";
  if (result.isNew) return "#60a5fa";
  if (result.passed) return "#4ade80";
  return "#f87171";
}
</script>

<template>
  <div class="vrt-panel">
    <div class="vrt-header">
      <h3 class="vrt-title">
        Visual Regression Testing
      </h3>
      <div class="vrt-actions">
        <label class="vrt-update-label">
          <input v-model="updateSnapshots" class="vrt-checkbox" type="checkbox" />
          Update snapshots
        </label>
        <button class="vrt-run-btn" type="button" :disabled="isRunning" @click="runTest">
          <MdiIcon v-if="isRunning" class="spin" :path="mdiLoading" :size="14" />
          <MdiIcon v-else :path="mdiImageOutline" :size="14" />
          {{ isRunning ? "Running..." : "Run VRT" }}
        </button>
      </div>
    </div>
    <div v-if="!hasRun" class="vrt-empty">
      <p>
        Click "Run VRT" to capture and compare screenshots.
      </p>
      <p class="vrt-hint">
        Requires Playwright to be installed.
      </p>
    </div>
    <div v-else-if="error" class="vrt-error">
      <p>
        {{ error }}
      </p>
      <p class="vrt-hint">
        Make sure Playwright is installed:
        <code>
          npm install playwright
        </code>
      </p>
    </div>
    <template v-else>
      <div v-if="summary" class="vrt-summary">
        <div class="vrt-stat total">
          <span class="vrt-stat-value">
            {{ summary.total }}
          </span>
          <span class="vrt-stat-label">
            Total
          </span>
        </div>
        <div class="vrt-stat passed">
          <span class="vrt-stat-value">
            {{ summary.passed }}
          </span>
          <span class="vrt-stat-label">
            Passed
          </span>
        </div>
        <div v-if="summary.failed > 0" class="vrt-stat failed">
          <span class="vrt-stat-value">
            {{ summary.failed }}
          </span>
          <span class="vrt-stat-label">
            Failed
          </span>
        </div>
        <div v-if="summary.new > 0" class="vrt-stat new">
          <span class="vrt-stat-value">
            {{ summary.new }}
          </span>
          <span class="vrt-stat-label">
            New
          </span>
        </div>
      </div>
      <div class="vrt-results">
        <div
          v-for="(variantResults, variantName) in groupedResults"
          :key="variantName"
          class="vrt-variant"
        >
          <div class="vrt-variant-name">
            {{ variantName }}
          </div>
          <div class="vrt-viewports">
            <div
              v-for="result in variantResults"
              :key="result.viewport"
              class="vrt-viewport"
              :class="getStatusIcon(result)"
            >
              <span class="vrt-viewport-name">
                {{ result.viewport }}
              </span>
              <span class="vrt-status" :style="{ color: getStatusColor(result) }">
                <template v-if="result.error">
                  Error
                </template>
                <template v-else-if="result.isNew">
                  New
                </template>
                <template v-else-if="result.passed">
                  Pass
                </template>
                <template v-else>
                  Diff {{ result.diffPercentage?.toFixed(2) }}%
                </template>
              </span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.vrt-panel {
  padding: .5rem;
}

.vrt-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: .5rem;
}

.vrt-title {
  font-size: .875rem;
  font-weight: 600;
}

.vrt-actions {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.vrt-update-label {
  display: flex;
  align-items: center;
  gap: .375rem;
  font-size: .75rem;
  color: var(--musea-text-muted);
  cursor: pointer;
}

.vrt-checkbox {
  width: 14px;
  height: 14px;
  cursor: pointer;
}

.vrt-run-btn {
  display: flex;
  align-items: center;
  gap: .375rem;
  padding: .375rem .75rem;
  background: var(--musea-accent);
  border: none;
  border-radius: var(--musea-radius-sm);
  color: #fff;
  font-size: .75rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.vrt-run-btn:hover:not(:disabled) {
  background: var(--musea-accent-hover);
}

.vrt-run-btn:disabled {
  opacity: .5;
  cursor: not-allowed;
}

.vrt-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: .875rem;
}

.vrt-hint {
  font-size: .75rem;
  margin-top: .5rem;
  opacity: .7;
}

.vrt-hint code {
  background: var(--musea-bg-tertiary);
  padding: .125rem .375rem;
  border-radius: 3px;
  font-family: var(--musea-font-mono);
}

.vrt-error {
  padding: 1rem;
  background: #f871711a;
  border: 1px solid #f8717133;
  border-radius: var(--musea-radius-sm);
  color: #f87171;
  font-size: .8125rem;
}

.vrt-summary {
  display: flex;
  gap: .75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.vrt-stat {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: .5rem .75rem;
  text-align: center;
  min-width: 60px;
}

.vrt-stat-value {
  display: block;
  font-size: 1.25rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.vrt-stat-label {
  font-size: .625rem;
  color: var(--musea-text-muted);
  text-transform: uppercase;
  letter-spacing: .05em;
}

.vrt-stat.passed .vrt-stat-value {
  color: #4ade80;
}

.vrt-stat.failed .vrt-stat-value {
  color: #f87171;
}

.vrt-stat.new .vrt-stat-value {
  color: #60a5fa;
}

.vrt-results {
  display: flex;
  flex-direction: column;
  gap: .5rem;
}

.vrt-variant {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: .75rem;
}

.vrt-variant-name {
  font-weight: 600;
  font-size: .8125rem;
  margin-bottom: .5rem;
}

.vrt-viewports {
  display: flex;
  gap: .5rem;
  flex-wrap: wrap;
}

.vrt-viewport {
  display: flex;
  align-items: center;
  gap: .5rem;
  padding: .25rem .5rem;
  background: var(--musea-bg-tertiary);
  border-radius: var(--musea-radius-sm);
  font-size: .75rem;
}

.vrt-viewport-name {
  color: var(--musea-text-secondary);
}

.vrt-status {
  font-weight: 600;
  font-size: .6875rem;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.spin {
  animation: 1s linear infinite spin;
}
</style>
