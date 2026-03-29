import { useLocation } from "@solidjs/router";
import { createEffect, createMemo, onCleanup } from "solid-js";
import { useApi, useApi2, useChannels2 } from "@/api";
import { useCurrentUser } from "../contexts/currentUser.tsx";
import { generateFavicon } from "../drawing.ts";

export function useFavicon() {
	const api2 = useApi2();
	const channels2 = useChannels2();
	const store = useApi2();
	const location = useLocation();

	const totalMentions = createMemo(() => {
		let count = 0;
		for (const channel of [...channels2.cache.values()]) {
			if (channel.mention_count && channel.mention_count > 0) {
				count += channel.mention_count;
			}
		}
		return count;
	});

	const faviconData = createMemo(() => {
		const path = location.pathname;
		const roomMatch = path.match(/^\/room\/([^/]+)/);
		if (roomMatch) {
			const room = store.rooms.cache.get(roomMatch[1]);
			if (room) return { type: "room" as const, room };
		}

		const channelMatch = path.match(/^\/(?:channel|thread)\/([^/]+)/);
		if (channelMatch) {
			const channel = channels2.cache.get(channelMatch[1]);
			if (channel) {
				if (channel.type === "Dm") {
					const self = useCurrentUser();
					const selfUser = self();
					if (selfUser) {
						const otherUser = channel.recipients?.find(
							(i) => i.id !== selfUser.id,
						);
						if (otherUser) {
							return { type: "user" as const, user: otherUser };
						}
					}
				}
				return { type: "channel" as const, channel };
			}
		}
		return null;
	});

	createEffect(() => {
		const mentions = totalMentions();
		const data = faviconData();
		let oldUrl: string | null = null;

		(async () => {
			const blob = await generateFavicon(mentions, data ?? undefined);
			if (!blob) return;

			const url = URL.createObjectURL(blob);
			let link: HTMLLinkElement | null =
				document.querySelector("link[rel~='icon']");
			if (!link) {
				link = document.createElement("link");
				link.rel = "icon";
				document.head.appendChild(link);
			}
			oldUrl = link.href;
			link.href = url;
			if (oldUrl && oldUrl.startsWith("blob:")) {
				URL.revokeObjectURL(oldUrl);
			}
		})();

		onCleanup(() => {
			if (oldUrl && oldUrl.startsWith("blob:")) {
				URL.revokeObjectURL(oldUrl);
			}
		});
	});
}
