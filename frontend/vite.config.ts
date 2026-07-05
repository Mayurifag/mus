import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  optimizeDeps: {
    include: [
      "music-metadata",
      "@lucide/svelte",
      "@tanstack/svelte-virtual",
      "bits-ui",
    ],
  },
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      include: ["src/lib/**/*.{ts,svelte}", "src/routes/**/*.{ts,svelte}"],
      exclude: [
        "src/**/*.test.{ts,svelte}",
        "src/**/*.spec.{ts,svelte}",
        "src/**/__tests__",
        "src/app.d.ts",
        "src/vite-env.d.ts",
      ],
    },
    projects: [
      {
        extends: "./vite.config.ts",
        plugins: [svelteTesting()],
        test: {
          name: "client",
          environment: "jsdom",
          clearMocks: true,
          include: ["src/**/*.{test,spec}.{js,ts}"],
          exclude: ["src/**/*.server.{test,spec}.{js,ts}"],
          setupFiles: ["./vitest-setup-client.ts"],
        },
      },
      {
        extends: "./vite.config.ts",
        test: {
          name: "server",
          environment: "node",
          include: ["src/**/*.server.{test,spec}.{js,ts}"],
        },
      },
    ],
  },
});
