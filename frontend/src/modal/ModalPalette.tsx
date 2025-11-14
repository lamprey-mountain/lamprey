import { useNavigate } from "@solidjs/router";
import type { Channel, Room } from "sdk";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { useApi } from "../api";
import { useCtx } from "../context";
import { getThumbFromId } from "../media/util";
import { ChannelIcon } from "../User";
import { Modal } from "./mod";
import { useModals } from "../contexts/modal";

export const ModalPalette = () => {
	const api = useApi();
	const ctx = useCtx();
	const navigate = useNavigate();
	const [, modalCtl] = useModals();

	const [query, setQuery] = createSignal("");
	const [selectedIndex, setSelectedIndex] = createSignal(0);

	// try to load all threads
	const rooms = api.rooms.list();
	api.dms.list();
	createEffect(() => {
		for (const room of rooms()?.items ?? []) {
			api.channels.list(() => room.id);
		}
	});

	type PaletteItem = {
		type: "room" | "thread" | "link";
		id: string;
		name: string;
		action: () => void;
		channel?: Channel;
		room?: Room;
	};

	const allItems = createMemo((): PaletteItem[] => {
		const rooms = [...api.rooms.cache.values()].map((room) => ({
			type: "room" as const,
			id: room.id,
			name: room.name,
			action: () => navigate(`/room/${room.id}`),
			room: room,
		}));
		const threads = [...api.channels.cache.values()].map((thread) => ({
			type: "thread" as const,
			id: thread.id,
			name: thread.name,
			action: () => navigate(`/channel/${thread.id}`),
			channel: thread,
		}));

		const staticItems: PaletteItem[] = [
			{
				type: "link" as const,
				id: "home",
				name: "home",
				action: () => navigate("/"),
			},
			{
				type: "link" as const,
				id: "inbox",
				name: "inbox",
				action: () => navigate("/inbox"),
			},
			{
				type: "link" as const,
				id: "friends",
				name: "friends",
				action: () => navigate("/friends"),
			},
			{
				type: "link" as const,
				id: "settings",
				name: "settings",
				action: () => navigate("/settings"),
			},
		];

		return [...staticItems, ...rooms, ...threads];
	});

	const recentChannels = createMemo(() => {
		return ctx.recentChannels().slice(1).map((i: any) =>
			api.channels.cache.get(i)!
		)
			.map((
				thread: any,
			) => ({
				type: "thread" as const,
				id: thread.id,
				name: thread.name,
				action: () => navigate(`/thread/${thread.id}`),
				channel: thread,
			}));
	});

	const filteredItems = createMemo(() => {
		const q = query().toLowerCase();
		if (!q) {
			return recentChannels();
		}
		return allItems().filter((item) =>
			item.name && item.name.toLowerCase().includes(q)
		);
	});

	createEffect(() => {
		setSelectedIndex(0);
	});

	const handleKeyDown = (e: KeyboardEvent) => {
		const len = filteredItems().length;
		if (len === 0) return;

		if (e.key === "ArrowDown") {
			e.preventDefault();
			setSelectedIndex((i) => (i + 1) % len);
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			setSelectedIndex((i) => (i - 1 + len) % len);
		} else if (e.key === "Enter") {
			e.preventDefault();
			const item = filteredItems()[selectedIndex()];
			if (item) {
				item.action();
				modalCtl.close();
			}
		}
	};

	const close = () => {
		modalCtl.close();
	};

	return (
		<Modal>
			<div onKeyDown={handleKeyDown} class="palette">
				<h3 class="dim">palette</h3>
				<input
					type="text"
					autofocus
					ref={(a) => queueMicrotask(() => a.focus())}
					value={query()}
					onInput={(e) => setQuery(e.currentTarget.value)}
					placeholder="type to search..."
				/>
				<div class="items">
					<For each={filteredItems().slice(0, 10)}>
						{(item, i) => (
							<div
								class="item"
								classList={{ selected: i() === selectedIndex() }}
								onClick={() => {
									item.action();
									close();
								}}
								onMouseEnter={() => setSelectedIndex(i())}
							>
								<Show when={item.type === "thread" && item.channel} keyed>
									<div class="item-icon">
										<ChannelIcon channel={item.channel!} />
									</div>
								</Show>
								<Show when={item.type === "room" && item.room} keyed>
									<div class="item-icon">
										<Show
											when={item.room!.icon}
											fallback={
												<div class="avatar fake">
													{item.room!.name.substring(0, 2)}
												</div>
											}
										>
											<img
												src={getThumbFromId(item.room!.icon!, 64)}
												class="avatar"
											/>
										</Show>
									</div>
								</Show>
								<span>{item.name}</span>
							</div>
						)}
					</For>
				</div>
			</div>
		</Modal>
	);
};
