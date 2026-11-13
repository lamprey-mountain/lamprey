import { createEffect, createSignal, onCleanup } from "solid-js";
import { useCtx } from "@/app/context";
import {
	applyTheme,
	colorSchemeQuery,
	resolveTheme,
	systemPrefersDark,
} from "@/lib/theme";

/** react to the system color scheme setting */
export function useSystemPrefersDark() {
	const [prefersDark, setPrefersDark] = createSignal(systemPrefersDark());

	const media = window.matchMedia?.(colorSchemeQuery);
	if (media) {
		const onChange = (e: MediaQueryListEvent) => setPrefersDark(e.matches);
		media.addEventListener("change", onChange);
		onCleanup(() => media.removeEventListener("change", onChange));
	}

	return prefersDark;
}

/** apply theme, message spacing, and chat font scale to the document */
export function useAppearance() {
	const ctx = useCtx();
	const systemDark = useSystemPrefersDark();

	createEffect(() => {
		const { scheme, contrast } = resolveTheme(
			ctx.preferences().frontend.theme,
			systemDark(),
		);
		applyTheme(scheme, contrast);
	});

	createEffect(() => {
		const frontend = ctx.preferences().frontend;
		const style = document.documentElement.style;
		const spacing = frontend.message_spacing;
		const fontScale = frontend.chat_font_scale;

		style.setProperty(
			"--message-spacing",
			`${Number.isFinite(spacing) ? spacing : 8}px`,
		);
		// unitless so it can be multiplied against em/rem values
		style.setProperty(
			"--chat-font-scale",
			String((Number.isFinite(fontScale) ? fontScale : 100) / 100),
		);
	});
}
