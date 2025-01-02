import { defineConfig } from "vite";
import deno from "@deno/vite-plugin";
import solid from "vite-plugin-solid";
import tailwindcss from "tailwindcss";

// https://vite.dev/config/
export default defineConfig({
	plugins: [deno(), solid()],
	css: {
		postcss: {
			plugins: [tailwindcss()],
		},
	},
	server: {
		watch: {
			// watching seems broken on my machine unfortunately
			usePolling: true
		}
	}
});
