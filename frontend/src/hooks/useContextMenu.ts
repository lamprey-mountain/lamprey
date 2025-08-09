import type { Setter } from "solid-js";
import { useApi } from "../api.tsx";
import type { Menu } from "../context.ts";

export function useContextMenu(setMenu: Setter<Menu | null>) {
	const api = useApi();

	const handleContextMenu = (e: MouseEvent) => {
		console.log("ctx menu");
		const targetEl = e.target as HTMLElement;

		const menuEl = targetEl.closest(
			".menu-room, .menu-thread, .menu-message, .menu-user",
		) as HTMLElement | null;
		const mediaEl = targetEl.closest(
			"a:not(.nav), img:not(.avatar), video, audio",
		) as
			| HTMLElement
			| null;
		console.log({ menuEl, mediaEl, targetEl });
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
			if (thread_id) {
				const thread = api.threads.cache.get(thread_id);
				if (!thread) return;
				menu = {
					type: "member_thread",
					thread_id: thread.id,
					user_id,
				};
			} else if (room_id) {
				const room = api.rooms.cache.get(room_id);
				if (!room) return;
				menu = {
					type: "member_room",
					room_id: room.id,
					user_id,
				};
			} else {
				menu = {
					type: "user",
					user_id,
				};
			}
		}

		if (menu) {
			e.preventDefault();
			setMenu({
				x: e.clientX,
				y: e.clientY,
				...menu,
			} as Menu);
		}
	};

	return { handleContextMenu };
}
