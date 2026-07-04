import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Vitest configuration for component tests.
//
// Mirrors the Vite setup the app uses (SvelteKit plugin + Svelte 5) so the
// components compile with the same compiler options as in production. The DOM
// environment is provided by jsdom via the setup file.
//
// The `$lib` alias is normally provided by the SvelteKit Vite plugin, but the
// component tests use the standalone Svelte plugin, so the alias is replicated
// here to match SvelteKit's generated tsconfig paths.
export default defineConfig({
  plugins: [svelte({ hot: false })],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/tests/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,js}"],
  },
  resolve: {
    conditions: ["browser"],
    alias: {
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
    },
  },
});
