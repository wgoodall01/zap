import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import { resolve } from "node:path";
import EnvironmentPlugin from "vite-plugin-environment";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    TanStackRouterVite({ autoCodeSplitting: true }),
    viteReact(),
    EnvironmentPlugin(["TG_MOCK_INIT_DATA"]),
  ],

  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },

  server: {
    allowedHosts: ["localhost", "dynamic-evolved-magpie.ngrok-free.app"],
  },

  // Find .env files in the project root.
  envDir: resolve(__dirname, "../.."),
});
