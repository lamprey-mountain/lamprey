import { onCleanup } from "solid-js";
import { useModals } from "../contexts/modal";
import { useCtx } from "../context.ts";
import { useContextMenu } from "./useContextMenu.ts";

interface UseGlobalEventHandlersOptions {
	setMenu: (menu: any) => void;
}

export function useGlobalEventHandlers(options: UseGlobalEventHandlersOptions) {
	const ctx = useCtx();
	const [modals, modalCtl] = useModals();
	const { handleContextMenu } = useContextMenu(options.setMenu);

	const handleClick = (e: MouseEvent) => {
		options.setMenu(null);
		ctx.setUserView(null);
		ctx.setThreadsView(null);
		if (!e.isTrusted) return;
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") {
			if (modals.length) {
				modalCtl.close();
			}
		} else if (e.key === "k" && e.ctrlKey) {
			e.preventDefault();
			if (modals.length) {
				modalCtl.close();
			} else {
				modalCtl.open({ type: "palette" });
			}
		} else if (e.key === "f" && e.ctrlKey) {
			e.preventDefault();
			const searchInput = document.querySelector(
				".search-input .ProseMirror",
			) as HTMLElement | null;
			searchInput?.focus();
		}
	};

	window.addEventListener("keydown", handleKeypress);
	window.addEventListener("click", handleClick);
	window.addEventListener("contextmenu", handleContextMenu);

	onCleanup(() => {
		window.removeEventListener("keydown", handleKeypress);
		window.removeEventListener("click", handleClick);
		window.removeEventListener("contextmenu", handleContextMenu);
	});

	return { handleContextMenu, handleClick, handleKeypress };
}
