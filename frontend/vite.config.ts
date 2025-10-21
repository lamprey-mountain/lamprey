import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import pkg from "./package.json";
import { execSync, spawnSync } from "node:child_process";

function getGitCommit() {
	if (process.env.VITE_GIT_SHA) {
		return process.env.VITE_GIT_SHA;
	}
	try {
		return execSync("git rev-parse HEAD").toString().trim();
	} catch {
		return "unknown";
	}
}

function isGitDirty() {
	if (process.env.VITE_GIT_DIRTY !== undefined) {
		return process.env.VITE_GIT_DIRTY === "true";
	}
	try {
		return spawnSync("git diff-index --quiet HEAD --").status !== 0;
	} catch {
		return false;
	}
}

// https://vite.dev/config/
export default defineConfig({
	define: {
		__VITE_PACKAGE_JSON__: pkg,
		__VITE_GIT_COMMIT__: JSON.stringify(getGitCommit()),
		__VITE_GIT_DIRTY__: isGitDirty(),
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
