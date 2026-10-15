import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The pages are built into iw-helper-gui.exe as they are: relative paths, nothing to download.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
