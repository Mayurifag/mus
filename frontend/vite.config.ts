import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import svgo from "vite-plugin-svgo";

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
  ],
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
    workspace: [
      {
        extends: "./vite.config.ts",
        plugins: [svelteTesting()],
        test: {
          name: "client",
          environment: "jsdom",
          clearMocks: true,
          include: ["src/**/*.svelte.{test,spec}.{js,ts}"],
          exclude: ["src/lib/server/**"],
          setupFiles: ["./vitest-setup-client.ts"],
        },
      },
      {
        extends: "./vite.config.ts",
        test: {
          name: "server",
          environment: "node",
          include: ["src/**/*.{test,spec}.{js,ts}"],
          exclude: ["src/**/*.svelte.{test,spec}.{js,ts}"],
        },
      },
    ],
  },
});
