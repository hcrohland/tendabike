import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  build: {
    target: "es2022",
  },
  server: {
    proxy: {
      "^/(user)|(strava)|(activ)": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
      "/types": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
      "/part": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
      "^/service.*": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
    },
  },
});
