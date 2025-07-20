import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import svgo from "vite-plugin-svgo";
import viteCompression from "vite-plugin-compression";

export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit(),
    svgo({
      multipass: true,
      plugins: [
        {
          name: "preset-default",
          params: {
            overrides: {
              cleanupIds: false, // Keep IDs for gradients/filters
              removeViewBox: false, // Keep viewBox for scaling
            },
          },
        },
      ],
    }),
    viteCompression({ algorithm: "gzip" }),
    viteCompression({ algorithm: "brotliCompress", ext: ".br" }),
  ],
  optimizeDeps: {
    include: [
      "music-metadata",
      "@lucide/svelte",
      "@tanstack/svelte-virtual",
      "bits-ui",
      "@castlenine/svelte-qrcode",
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
