import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

// The desktop shell (Tauri) and the browser shell load the same bundle.
// Tauri sets TAURI_DEV_HOST when driving the dev server; otherwise this is
// a plain browser dev server talking to the axum core over HTTP+SSE.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  server: {
    port: 5180,
    strictPort: true,
  },
  // Top-level await in main.tsx (shell client selection) needs a target
  // that supports it; these baselines cover every shell we ship to.
  build: {
    target: ["es2022", "chrome111", "edge111", "firefox111", "safari16"],
  },
});
