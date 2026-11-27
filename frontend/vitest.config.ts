import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

const nodeTests = "src/**/*.{node,server}.{test,spec}.{js,ts}";

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      clearMocks: true,
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
          plugins: [svelteTesting()],
          test: {
            name: "client",
            environment: "jsdom",
            // Reuse jsdom workers while keeping a fresh VM and window for each file.
            pool: "vmForks",
            include: ["src/**/*.{test,spec}.{js,ts}"],
            exclude: [nodeTests],
            setupFiles: ["./vitest-setup-client.ts"],
          },
        },
        {
          test: {
            name: "server",
            environment: "node",
            include: [nodeTests],
          },
        },
      ],
    },
  }),
);
