<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import {
  mdiMagnify,
  mdiWeatherSunny,
  mdiWeatherNight,
  mdiThemeLightDark,
  mdiChevronLeft,
  mdiChevronRight,
} from "@mdi/js";
import { useArts } from "../composables/useArts";
import { useSearch } from "../composables/useSearch";
import { useTheme } from "../composables/useTheme";
import SearchBar from "./SearchBar.vue";
import Sidebar from "./Sidebar.vue";
import SearchModal from "./SearchModal.vue";
import MdiIcon from "./MdiIcon.vue";

const router = useRouter();
const { arts, load } = useArts();
const { query, results } = useSearch(arts);
const { currentTheme, cycleTheme } = useTheme();

const searchModalOpen = ref(false);
const sidebarCollapsed = ref(false);

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}

const themeIcon = computed(() => {
  switch (currentTheme.value) {
    case "dark":
      return mdiWeatherNight;
    case "system":
      return mdiThemeLightDark;
    default:
      return mdiWeatherSunny;
  }
});

const themeLabel = computed(() => {
  switch (currentTheme.value) {
    case "dark":
      return "Dark";
    case "system":
      return "System";
    default:
      return "Light";
  }
});

// Global keyboard shortcuts
const handleKeydown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "k") {
    e.preventDefault();
    searchModalOpen.value = !searchModalOpen.value;
  }
  if ((e.metaKey || e.ctrlKey) && e.key === "b") {
    e.preventDefault();
    toggleSidebar();
  }
};

onMounted(() => {
  load();
  document.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});

const handleSearchSelect = (art: { path: string }, variantName?: string) => {
  router.push({ name: "component", params: { path: art.path } });
};
</script>

<template>
  <div class="gallery-layout">
    <header class="header">
      <div class="header-left">
        <router-link class="logo" to="/">
          <svg class="logo-svg" fill="none" height="32" viewBox="0 0 200 200" width="32">
            <g transform="translate(30, 25) scale(1.2)">
              <g transform="translate(15, 10) skewX(-15)">
                <path d="M 65 0 L 40 60 L 70 20 L 65 0 Z" fill="currentColor" />
                <path d="M 20 0 L 40 60 L 53 13 L 20 0 Z" fill="currentColor" />
              </g>
            </g>
            <g transform="translate(110, 120)">
              <line
                stroke="currentColor"
                stroke-linecap="round"
                stroke-width="3"
                x1="5"
                x2="5"
                y1="10"
                y2="50"
               />
              <line
                stroke="currentColor"
                stroke-linecap="round"
                stroke-width="3"
                x1="60"
                x2="60"
                y1="10"
                y2="50"
               />
              <path
                d="M 0 10 L 32.5 0 L 65 10"
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2.5"
               />
              <rect
                fill="none"
                height="12"
                opacity="0.7"
                rx="1"
                stroke="currentColor"
                stroke-width="1.5"
                width="14"
                x="15"
                y="18"
               />
              <rect
                fill="none"
                height="12"
                opacity="0.7"
                rx="1"
                stroke="currentColor"
                stroke-width="1.5"
                width="14"
                x="36"
                y="18"
               />
              <rect
                fill="none"
                height="12"
                opacity="0.6"
                rx="1"
                stroke="currentColor"
                stroke-width="1.5"
                width="18"
                x="23"
                y="35"
               />
            </g>
          </svg>
          Musea
        </router-link>
        <span class="header-subtitle">
          Component Gallery
        </span>
      </div>
      <div class="header-center">
        <button class="search-trigger" type="button" @click="searchModalOpen = true">
          <MdiIcon class="search-icon" :path="mdiMagnify" :size="16" />
          <span>
            Search components...
          </span>
          <kbd>
            ⌘K
          </kbd>
        </button>
      </div>
      <div class="header-right">
        <button
          class="theme-toggle"
          type="button"
          :title="`Theme: ${themeLabel}`"
          @click="cycleTheme"
        >
          <MdiIcon :path="themeIcon" :size="18" />
        </button>
      </div>
    </header>
    <main class="main" sidebar-collapsed": sidebarCollapsed }" :class="{ ">
      <!-- Sidebar -->
      <aside class="sidebar-wrapper" :class="{ collapsed: sidebarCollapsed }">
        <Sidebar v-show="!sidebarCollapsed" :arts="results" />
        <button
          "Collapse
          (⌘B)"
          (⌘B)""
          class="sidebar-toggle"
          Expand
          sidebar
          sidebar
          type="button"
          :
          :title="sidebarCollapsed ? "
          @click="toggleSidebar"
        >
          <MdiIcon :path="sidebarCollapsed ? mdiChevronRight : mdiChevronLeft" :size="16" />
        </button>
      </aside>
      <!-- Main Content -->
      <section class="content">
        <router-view />
      </section>
    </main>
    <!-- Search Modal -->
    <SearchModal
      :arts="arts"
      :is-open="searchModalOpen"
      @close="searchModalOpen = false"
      @select="handleSearchSelect"
     />
  </div>
</template>

<style scoped>
.gallery-layout {
  height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  background: var(--musea-bg-secondary);
  border-bottom: 1px solid var(--musea-border);
  padding: 0 1.5rem;
  height: var(--musea-header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}

.header-center {
  flex: 1;
  display: flex;
  justify-content: center;
  max-width: 400px;
}

.logo {
  display: flex;
  align-items: center;
  gap: .5rem;
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--musea-accent);
  text-decoration: none;
}

.logo-svg {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
}

.header-subtitle {
  color: var(--musea-text-muted);
  font-size: .8125rem;
  font-weight: 500;
  padding-left: 1.5rem;
  border-left: 1px solid var(--musea-border);
}

.search-trigger {
  display: flex;
  align-items: center;
  gap: .5rem;
  width: 100%;
  padding: .5rem .75rem;
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  color: var(--musea-text-muted);
  font-size: .875rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.search-trigger:hover {
  border-color: var(--musea-accent);
  color: var(--musea-text-secondary);
}

.search-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.search-trigger span {
  flex: 1;
  text-align: left;
}

.search-trigger kbd {
  padding: .125rem .375rem;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  font-size: .75rem;
  font-family: var(--musea-font-mono);
}

.header-right {
  display: flex;
  align-items: center;
}

.theme-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.theme-toggle:hover {
  border-color: var(--musea-accent);
  color: var(--musea-text);
}

.main {
  display: grid;
  grid-template-columns: var(--musea-sidebar-width) 1fr;
  flex: 1;
  overflow: hidden;
  height: calc(100vh - var(--musea-header-height));
  transition: grid-template-columns .2s;
}

.main.sidebar-collapsed {
  grid-template-columns: 40px 1fr;
}

.sidebar-wrapper {
  height: 100%;
  max-height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
  background: var(--musea-bg-secondary);
  border-right: 1px solid var(--musea-border);
}

.sidebar-wrapper.collapsed {
  overflow: hidden;
}

.sidebar-wrapper :deep(.sidebar) {
  border-right: none;
}

.sidebar-toggle {
  position: absolute;
  bottom: .75rem;
  right: .75rem;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
  z-index: 10;
}

.sidebar-wrapper.collapsed .sidebar-toggle {
  right: auto;
  left: 50%;
  transform: translateX(-50%);
}

.sidebar-toggle:hover {
  background: var(--musea-bg-elevated);
  color: var(--musea-text);
  border-color: var(--musea-text-muted);
}

.content {
  background: var(--musea-bg-primary);
  overflow-y: auto;
  height: calc(100vh - var(--musea-header-height));
}

@media (width <= 768px) {
  .main {
    grid-template-columns: 1fr !important;
  }

  .sidebar-wrapper {
    display: none;
  }

  .header-subtitle {
    display: none;
  }

  .header-center {
    display: none;
  }
}
</style>
