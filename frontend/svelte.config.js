import adapter from "@sveltejs/adapter-auto";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),

  kit: {
    adapter: adapter(),
    alias: {
      $components: "./src/lib/components",
      $stores: "./src/lib/stores",
      $services: "./src/lib/services",
      $types: "./src/lib/types",
    },
  },
};

export default config;
