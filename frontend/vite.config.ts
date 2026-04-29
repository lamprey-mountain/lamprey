import { execSync, spawnSync } from "node:child_process";
import path from "node:path";
import { defineConfig, searchForWorkspaceRoot } from "vite";
import solid from "vite-plugin-solid";
import pkg from "./package.json";

const readEnv = (v: string) => {
	const val = process.env[v];
	if (val) {
		return val;
	} else {
		console.error(
			`Error: ${v} environment variable is required (are you using \`nix develop\`?)`,
		);
		process.exit(1);
	}
};

const WASM_MARKDOWN_PKG = readEnv("WASM_MARKDOWN_PKG");
const TWEMOJI_SPRITESHEETS = readEnv("TWEMOJI_SPRITESHEETS");

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
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
			"@wasm-markdown": path.resolve(__dirname, WASM_MARKDOWN_PKG),
			"@twemoji-spritesheets": path.resolve(__dirname, TWEMOJI_SPRITESHEETS),
		},
	},
	define: {
		__VITE_PACKAGE_JSON__: pkg,
		__VITE_GIT_COMMIT__: JSON.stringify(getGitCommit()),
		__VITE_GIT_DIRTY__: isGitDirty(),
	},
	plugins: [
		solid(),
		{
			name: "sw-manifest",
			generateBundle(_, bundle) {
				const assets = Object.values(bundle)
					.filter((chunk) => chunk.type === "chunk" || chunk.type === "asset")
					.filter((chunk) => !chunk.fileName.endsWith(".map"))
					.map((chunk) => "/" + chunk.fileName);

				for (const chunk of Object.values(bundle)) {
					if (chunk.type === "chunk" && chunk.facadeModuleId?.includes("sw.")) {
						chunk.code = chunk.code.replace(
							"__PRECACHE_MANIFEST__",
							JSON.stringify(assets),
						);
					}
				}
			},
		},
	],
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true,
		},
		fs: {
			allow: [
				searchForWorkspaceRoot(process.cwd()),
				path.resolve(WASM_MARKDOWN_PKG),
				path.resolve(TWEMOJI_SPRITESHEETS),
			],
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
