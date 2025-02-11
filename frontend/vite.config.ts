import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import { extname } from "node:path";
import * as sass from "sass";

const sassCompiler = sass.initCompiler();

// https://vite.dev/config/
export default defineConfig({
	plugins: [solid(), {
		name: "sass",
		load(path: string) {
			if (extname(path) !== ".scss") return;
			const compiled = sassCompiler.compile(path);
			return { code: compiled.css };
		},
	}],
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true,
		},
	},
	build: {
		sourcemap: true,
	},
	css: {
		preprocessorOptions: {
			scss: {
				api: "modern-compiler",
			},
		},
		// transformer: "lightningcss"
		// transformer: "postcss"
	},
});
