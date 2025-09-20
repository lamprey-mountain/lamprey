import { createEffect, createSignal, JSX, onCleanup, Show } from "solid-js";
import {
	clearDelegatedEvents,
	DelegatedEvents,
	delegateEvents,
	Portal,
} from "solid-js/web";

export const createPopup = (props: {
	width?: number;
	height?: number;
	title: () => string;
	content: () => JSX.Element;
}) => {
	const [popup, setPopup] = createSignal<Window | null>(null);

	// sync stylesheets for hot module reloading during dev
	const observer = new MutationObserver(() => {
		const p = popup();
		if (!p) return;

		const popupSheets = p.document.querySelectorAll("style");
		const mainSheets = document.querySelectorAll("style");
		popupSheets.forEach((sheet, i) => {
			if (mainSheets[i]) {
				sheet.textContent = mainSheets[i].textContent;
			}
		});
	});

	observer.observe(document.head, {
		childList: true,
		subtree: true,
		characterData: true,
	});

	onCleanup(() => {
		popup()?.close();
		observer.disconnect();
	});

	createEffect(() => {
		const p = popup();
		if (p) {
			p.document.title = props.title() || "popup";
		}
	});

	return {
		show() {
			const pop = window.open(
				"",
				"_blank",
				`width=${props.width || 400},height=${props.height || 300}`,
			);
			if (!pop) return false;

			for (const el of Array.from(document.head.children)) {
				if (el.nodeName === "TITLE") continue;
				pop.document.head.append(el.cloneNode(true));
			}

			pop.addEventListener("unload", () => {
				clearDelegatedEvents(pop.document);
			});

			delegateEvents([...DelegatedEvents], pop.document);
			setPopup(pop);

			return true;
		},
		hide() {
			popup()?.close();
			setPopup(null);
		},
		visible() {
			const p = popup();
			return p !== null && !p.closed;
		},
		View() {
			return (
				<Show when={popup()}>
					{(p) => (
						<Portal mount={p().document.body}>
							{props.content()}
						</Portal>
					)}
				</Show>
			);
		},
	};
};
