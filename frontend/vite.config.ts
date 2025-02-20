import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// https://vite.dev/config/
export default defineConfig({
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
