import type { Preferences } from "sdk";

export type Theme = Preferences["frontend"]["theme"];
export type ColorScheme = "light" | "dark";
export type Contrast = "normal" | "high";

export const colorSchemeQuery = "(prefers-color-scheme: dark)";

/** read the system color scheme preference (false if matchMedia is unavailable) */
export function systemPrefersDark(): boolean {
	return window.matchMedia?.(colorSchemeQuery)?.matches ?? false;
}

/** resolve a theme preference into the scheme and contrast to render */
export function resolveTheme(
	theme: Theme | undefined,
	systemIsDark: boolean,
): { scheme: ColorScheme; contrast: Contrast } {
	switch (theme) {
		case "light":
			return { scheme: "light", contrast: "normal" };
		case "light-highcontrast":
			return { scheme: "light", contrast: "high" };
		case "dark":
			return { scheme: "dark", contrast: "normal" };
		case "dark-highcontrast":
			return { scheme: "dark", contrast: "high" };
		case "auto-highcontrast":
			return { scheme: systemIsDark ? "dark" : "light", contrast: "high" };
		case "auto":
		default:
			return { scheme: systemIsDark ? "dark" : "light", contrast: "normal" };
	}
}

/** write the resolved theme to the document root, which the palettes key off of */
export function applyTheme(scheme: ColorScheme, contrast: Contrast) {
	const style = document.documentElement.dataset;
	style.theme = scheme;
	style.contrast = contrast;
}
