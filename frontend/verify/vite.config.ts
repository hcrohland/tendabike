import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// Standalone Vite harness for component-level browser verification (see
// docs/component-verify.md). Deliberately not a SvelteKit app: no routes, no
// load functions, no backend — the app's real components and theme
// (src/app.css) on a fixed port. $app/* imports resolve to the stubs.
const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));

export default defineConfig({
  root: here("./"),
  plugins: [tailwindcss(), svelte({ preprocess: vitePreprocess() })],
  resolve: {
    conditions: ["browser"],
    alias: {
      "$app/state": here("./stubs/app-state.js"),
      "$app/navigation": here("./stubs/app-navigation.js"),
      "$app/page": here("./stubs/app-page.js"),
      "$app/stores": here("./stubs/app-stores.js"),
      "$app/environment": here("./stubs/app-environment.js"),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 5199,
    strictPort: true,
    fs: { allow: [here("../..")] },
  },
});
