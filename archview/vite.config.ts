import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

export default defineConfig(({ mode }) => ({
  // ssr:false forces the client Solid runtime even under Vitest's SSR
  // transform mode, so component tests can render JSX in jsdom.
  plugins: [solidPlugin({ ssr: false })],
  resolve: {
    // Vitest 2 (Vite 5) resolves solid-js/web through the `node`
    // condition → dist/server.js. `browser` must win so JSX client
    // helpers (template/insert) resolve to the DOM runtime. Scoped to
    // test mode so production builds keep their default conditions.
    ...(mode === "test"
      ? { conditions: ["browser", "development", "solid"] }
      : {}),
  },
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
    setupFiles: ["./src/__tests__/setup.ts"],
  },
}));
