import { defineConfig } from "vite";
import deno from "@deno/vite-plugin";
import solid from "vite-plugin-solid";

// https://vite.dev/config/
export default defineConfig({
	plugins: [deno(), solid()],
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true,
		},
	},
	build: {
		sourcemap: true,
	},
});
