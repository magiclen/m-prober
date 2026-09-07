import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The Rust side lists every asset by name (see `src/web/static_files.rs`), so the file names must not carry a content hash.
export default defineConfig(({ mode }) => ({
    base: "/",
    build: {
        outDir: "../front-end",
        // The favicons live in `front-end` and are not produced by this build.
        emptyOutDir: false,
        cssCodeSplit: false,
        sourcemap: mode === "development",
        rollupOptions: {
            output: {
                codeSplitting: false,
                entryFileNames: "js/bundle.js",
                assetFileNames: "css/bundle[extname]",
            },
        },
    },
    plugins: [react()],
    test: {
        environment: "jsdom",
        setupFiles: ["./src/test/setup.ts"],
    },
    resolve: {
        tsconfigPaths: true,
    },
    server: {
        host: "localhost",
        port: 5173,
        proxy: {
            "/api": {
                target: "http://127.0.0.1:8000",
                changeOrigin: false,
            },
        },
    },
    preview: {
        host: "localhost",
        port: 4173,
    },
}));
