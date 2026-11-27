import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

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
});
