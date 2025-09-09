import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import pkg from "./package.json";
import { execSync, spawnSync } from "node:child_process";

// https://vite.dev/config/
export default defineConfig({
	define: {
		__VITE_PACKAGE_JSON__: pkg,
		__VITE_GIT_COMMIT__: JSON.stringify(
			execSync("git rev-parse HEAD").toString().trim(),
		),
		__VITE_GIT_DIRTY__:
			spawnSync("git diff-index --quiet HEAD --").status !== 0,
	},
	plugins: [solid()],
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true,
		},
	},
	build: {
		sourcemap: true,
		rollupOptions: {
			input: {
				app: "./index.html",
				sw: "./sw.ts",
			},
			output: {
				entryFileNames: (info) => {
					if (info.name === "sw") return "[name].js";
					return "assets/js/[name]-[hash].js";
				},
			},
		},
	},
	css: {
		preprocessorOptions: {
			scss: {
				api: "modern-compiler",
			},
		},
	},
});
