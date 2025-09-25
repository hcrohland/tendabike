import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  build: { target: "es2022" },
  server: {
    proxy: {
      "^/(api)|(strava)": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
    },
  },
});
