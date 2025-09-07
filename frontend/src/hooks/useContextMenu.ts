import type { Setter } from "solid-js";
import { useApi } from "../api.tsx";
import type { Menu } from "../context.ts";

export function useContextMenu(setMenu: Setter<Menu | null>) {
	const api = useApi();

	const handleContextMenu = (e: MouseEvent) => {
		console.log("[menu] open context menu");
		const targetEl = e.target as HTMLElement;

		const menuEl = targetEl.closest(
			".menu-room, .menu-thread, .menu-message, .menu-user",
		) as HTMLElement | null;
		const mediaEl = targetEl.closest(
			"a:not(.nav), img:not(.avatar), video, audio",
		) as
			| HTMLElement
			| null;
		console.log("[menu] target elements", { menuEl, mediaEl, targetEl });
		if (!menuEl) return;
		if (mediaEl && targetEl !== menuEl) return;

		const getData = (key: string) => {
			const target = menuEl.closest(`[${key}]`) as HTMLElement | null;
			return target
				?.dataset[
					key.slice("data-".length).replace(
						/-([a-z])/g,
						(_, c) => c.toUpperCase(),
					)
				];
		};

		let menu: Partial<Menu> | null = null;
		const room_id = getData("data-room-id");
		const thread_id = getData("data-thread-id");
		const message_id = getData("data-message-id");
		const user_id = getData("data-user-id");
		console.log("[menu] menu id data", {
			room_id,
			thread_id,
			message_id,
			user_id,
		});

		if (menuEl.classList.contains("menu-room")) {
			if (!room_id) return;
			menu = {
				type: "room",
				room_id,
			};
		} else if (menuEl.classList.contains("menu-thread")) {
			if (!thread_id) return;
			menu = {
				type: "thread",
				thread_id,
			};
		} else if (menuEl.classList.contains("menu-message")) {
			const message = api.messages.cache.get(message_id!);
			if (!message) return;
			const thread_id = message.thread_id;
			const version_id = message.version_id;
			menu = {
				type: "message",
				thread_id,
				message_id,
				version_id,
			};
		} else if (menuEl.classList.contains("menu-user")) {
			if (!user_id) return;
			const thread = api.threads.cache.get(thread_id!);
			const room = api.rooms.cache.get(room_id!);
			if (thread?.room_id && room?.id && thread.room_id !== room.id) {
				console.warn("mismatched thread/room ids!");
			}

			menu = {
				type: "user",
				thread_id: thread?.id,
				room_id: thread?.room_id ?? room?.id ?? undefined,
				user_id,
			};
		}

		if (menu) {
			console.log("[menu] resolved menu", menu);
			e.preventDefault();
			setMenu({
				x: e.clientX,
				y: e.clientY,
				...menu,
			} as Menu);
		} else {
			console.log("[menu] no resolved menu");
		}
	};

	return { handleContextMenu };
}
