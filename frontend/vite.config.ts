import { paraglideVitePlugin } from "@inlang/paraglide-js";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./paraglide",
      strategy: ["cookie", "preferredLanguage", "baseLocale"],
    }),
    tailwindcss(),
    svelte(),
  ],

  test: {
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
    hmr: {
      host: "127.0.0.1",
    },
    proxy: {
      "^/(api)|(strava)": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
    },
  },
});
