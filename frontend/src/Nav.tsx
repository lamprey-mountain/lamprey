import { createEffect, createSignal, For, Show } from "solid-js";
import { A } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Thread } from "sdk";
import { flags } from "./flags.ts";
import { useVoice } from "./voice-provider.tsx";
import { useConfig } from "./config.tsx";

export const ThreadNav = (props: { room_id?: string }) => {
	const config = useConfig();
	const api = useApi();
	const [voice] = useVoice();

	// track drag indices
	const [dragging, setDragging] = createSignal<number | null>(null);
	const [target, setTarget] = createSignal<number | null>(null);

	// local list of threads for this room
	const [list, setList] = createSignal<Thread[]>([]);

	createEffect(() => {
		if (props.room_id) {
			api.threads.list(() => props.room_id!);
		} else {
			api.dms.list();
		}
	});

	const room = props.room_id
		? api.rooms.fetch(() => props.room_id!)
		: () => null;

	// update list when room changes
	createEffect(() => {
		const threads = [...api.threads.cache.values()]
			.filter((t) =>
				(props.room_id ? t.room_id === props.room_id : t.room_id === null) &&
				!t.deleted_at
			);
		if (props.room_id) {
			threads.sort((a, b) => a.id < b.id ? 1 : -1);
		} else {
			threads.sort((a, b) =>
				(a.last_version_id ?? "") < (b.last_version_id ?? "") ? 1 : -1
			);
		}
		setList(threads);
	});

	// helper to get index from the element's data-index
	const getIndex = (e: DragEvent) =>
		Number((e.currentTarget as HTMLElement).dataset.index ?? -1);

	const handleDragStart = (e: DragEvent) => {
		setDragging(getIndex(e));
	};

	const handleDragEnter = (e: DragEvent) => {
		e.preventDefault();
		setTarget(getIndex(e));
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const from = dragging();
		const to = target();
		if (from === null || to === null || from === to) return;

		const updated = [...list()];
		// splice out the dragged item
		const [moved] = updated.splice(from, 1);
		// insert it at the target index
		updated.splice(to, 0, moved);
		setList(updated);

		setDragging(null);
		setTarget(null);
	};

	return (
		<nav id="nav">
			<Show when={flags.has("nav_header")}>
				<header>
					{props.room_id ? (room()?.name ?? "loading...") : "home"}
				</header>
			</Show>

			<ul>
				<li>
					<A
						href={props.room_id ? `/room/${props.room_id}` : "/"}
						class="menu-thread"
						draggable={false}
						end
					>
						home
					</A>
				</li>

				<Show when={!props.room_id}>
					<Show when={flags.has("inbox")}>
						<li>
							<A
								href="/inbox"
								class="menu-thread"
								draggable={false}
								end
							>
								inbox
							</A>
						</li>
					</Show>
				</Show>

				<For each={list()}>
					{(thread, idx) => (
						<li
							data-index={idx()}
							draggable
							onDragStart={handleDragStart}
							onDragEnter={handleDragEnter}
							onDragOver={handleDragOver}
							onDrop={handleDrop}
							classList={{
								dragging: dragging() === idx(),
								over: target() === idx(),
								unread: thread.type !== "Voice" && !!thread.is_unread,
							}}
						>
							<ItemThread thread={thread} />
							<For
								each={[...api.voiceStates.values()].filter((i) =>
									i.thread_id === thread.id
								).sort((a, b) =>
									Date.parse(a.joined_at) - Date.parse(b.joined_at)
								)}
							>
								{(s) => {
									const user = api.users.fetch(() => s.user_id);
									const room_member = props.room_id
										? api.room_members.fetch(() => props.room_id!, () =>
											s.user_id)
										: () =>
											null;
									const name = () =>
										room_member()?.override_name || user()?.name ||
										"unknown user";
									// <svg viewBox="0 0 32 32" style="height:calc(1em + 4px);margin-right:8px" preserveAspectRatio="none">
									// 	<line x1={0} y1={0} x2={0} y2={32} stroke-width={4} style="stroke:white"/>
									// 	<line x1={0} y1={32} x2={32} y2={32} stroke-width={4} style="stroke:white"/>
									// </svg>

									return (
										<div
											class="voice-participant menu-user"
											classList={{
												speaking:
													((voice.rtc?.speaking.get(s.user_id)?.flags ?? 0) &
														1) === 1,
											}}
											data-thread-id={s.thread_id}
											data-user-id={s.user_id}
										>
											<Show
												when={user()?.avatar}
												fallback={<div class="fallback-avatar"></div>}
											>
												<img
													src={`${config.cdn_url}/thumb/${user()?.avatar}?size=64`}
												/>
											</Show>{" "}
											{name()}
										</div>
									);
								}}
							</For>
						</li>
					)}
				</For>
			</ul>
			<div style="margin: 8px" />
		</nav>
	);
};

const ItemThread = (props: { thread: Thread }) => {
	return (
		<A
			href={`/thread/${props.thread.id}`}
			class="menu-thread"
			classList={{
				closed: !!props.thread.archived_at,
				unread: props.thread.type !== "Voice" && !!props.thread.is_unread,
			}}
			data-thread-id={props.thread.id}
		>
			{props.thread.name}
		</A>
	);
};
