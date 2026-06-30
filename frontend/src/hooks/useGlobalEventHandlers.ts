import { onCleanup } from "solid-js";
import { useCtx } from "@/app/context";
import { useMenu, useUserPopout } from "@/contexts/mod.tsx";
import { useModals } from "@/contexts/modal";
import { useContextMenu } from "./useContextMenu.ts";

export function useGlobalEventHandlers() {
	const ctx = useCtx();
	const [modals, modalCtl] = useModals();
	const { menu, setMenu } = useMenu();
	const { setUserView } = useUserPopout();
	const { handleContextMenu } = useContextMenu(setMenu);

	const handleClick = (e: MouseEvent) => {
		// 1. close context menu if open
		if (menu()) {
			setMenu(null);
			return;
		}

		// TODO: implement as written
		// 2. close existing overlay (user view, reaction picker, etc...)
		// NOTE: or overlay*s*? how would multiple work?
		setUserView(null);
		ctx.setThreadsView(null);

		// TODO: implement as written
		// 3. close modal if background clicked
		// if (modals.length) {
		// 	modalCtl.close();
		// }

		// TODO: implement as written
		// 4. open user view
		// const target = (e.target as Element);
		// target.closest(".user-view");
		// data-user-id

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
