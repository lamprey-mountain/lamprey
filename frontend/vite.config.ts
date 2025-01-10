import { defineConfig } from "vite";
import deno from "@deno/vite-plugin";
import solid from "vite-plugin-solid";
import tailwindcss from "tailwindcss";
import { nodePolyfills } from "vite-plugin-node-polyfills";

// https://vite.dev/config/
export default defineConfig({
	plugins: [deno(), solid(), nodePolyfills()],
	css: {
		postcss: {
			plugins: [tailwindcss()],
		},
	},
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true
		},
    // headers: {
    //   'Cross-Origin-Opener-Policy': 'same-origin',
    //   'Cross-Origin-Embedder-Policy': 'require-corp',
    // },
    // fs: {
    // 	allow: [".", "../node_modules/.deno"],
    // },
	},
  // optimizeDeps: {
  //   exclude: ['@electric-sql/pglite'],
  // },
  // optimizeDeps: {
  //   exclude: ['@sqlite.org/sqlite-wasm'],
  // },
});
