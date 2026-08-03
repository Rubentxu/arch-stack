import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

export default defineConfig({
  plugins: [solidPlugin()],
  server: {
    port: 18080,
    strictPort: false,
  },
  build: {
    target: "esnext",
    sourcemap: true,
  },
  test: {
    environment: "node",
    globals: true,
  },
});
