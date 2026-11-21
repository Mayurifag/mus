import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit({
      preprocess: vitePreprocess(),
      adapter: adapter({ fallback: "index.html" }),
    }),
    {
      name: "name-hls-playback-chunk",
      apply: "build",
      config(config) {
        const output =
          config.environments?.client?.build?.rolldownOptions?.output;
        if (
          Array.isArray(output) ||
          typeof output?.chunkFileNames !== "function"
        )
          return;
        const chunkFileNames = output.chunkFileNames;
        output.chunkFileNames = (chunk) => {
          const path = chunkFileNames(chunk);
          return chunk.facadeModuleId?.endsWith("/hls.js/dist/hls.light.mjs")
            ? path.replace("[hash]", "hls-playback.[hash]")
            : path;
        };
      },
    },
  ],
  resolve: {
    alias: { $lib: fileURLToPath(new URL("./src/lib", import.meta.url)) },
  },
  optimizeDeps: {
    include: ["music-metadata", "bits-ui"],
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
