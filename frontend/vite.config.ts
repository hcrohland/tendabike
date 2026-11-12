import adapter from "@sveltejs/adapter-static";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { sveltekit } from "@sveltejs/kit/vite";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vitest/config";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./paraglide",
      strategy: ["cookie", "preferredLanguage", "baseLocale"],
    }),
    tailwindcss(),
    sveltekit({
      adapter: adapter({ fallback: "200.html" }),
      preprocess: vitePreprocess(),
    }),
  ],

  resolve: {
    conditions: ["browser"],
  },

  test: {
    fsModuleCache: true,
    environment: "jsdom",
    include: ["src/**/*.test.{ts,js}"],
    setupFiles: ["./src/test/setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**"],
      exclude: ["src/**/*.d.ts", "src/test/**"],
    },
  },

  build: {
    rolldownOptions: {
      checks: {
        pluginTimings: false, // disable the warning
      },
    },
    target: "es2022",
  },
  server: {
    host: "127.0.0.1",
    proxy: {
      "^/(api|strava|icu|source)": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
    },
  },
});
