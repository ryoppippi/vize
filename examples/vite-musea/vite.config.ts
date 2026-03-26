import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    ignorePatterns: ["dist/**", "playwright-report/**", "e2e/vrt/test-results/**"],
  },
  lint: {
    ignorePatterns: ["dist/**", "playwright-report/**", "e2e/vrt/test-results/**"],
  },
});
