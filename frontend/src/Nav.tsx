import { createEffect, createSignal, For, Match, Show, Switch } from "solid-js";
import { A, useParams } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Thread } from "sdk";
import { flags } from "./flags.ts";
import { useVoice } from "./voice-provider.tsx";
import { useConfig } from "./config.tsx";

export const ThreadNav = (props: { room_id?: string }) => {
	const config = useConfig();
	const api = useApi();
	const [voice] = useVoice();
	const params = useParams();

	// track drag ids
	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<string | null>(null);

	const [categories, setCategories] = createSignal<
		Array<{ category: Thread | null; threads: Array<Thread> }>
	>([]);

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
				props.room_id ? t.room_id === props.room_id : t.room_id === null
			);
		if (props.room_id) {
			// sort by id
			threads.sort((a, b) => {
				if (a.position === null && b.position === null) {
					return a.id < b.id ? 1 : -1;
				}
				if (a.position === null) return 1;
				if (b.position === null) return -1;
				return a.position! - b.position!;
			});
		} else {
			// sort by activity in dms list
			threads.sort((a, b) =>
				(a.last_version_id ?? "") < (b.last_version_id ?? "") ? 1 : -1
			);
		}

		const categories = new Map<string | null, Array<Thread>>();
		for (const t of threads) {
			if (t.type === "Category") {
				const cat = categories.get(t.id) ?? [];
				categories.set(t.id, cat);
			} else {
				const children = categories.get(t.parent_id!) ?? [];
				children.push(t);
				categories.set(t.parent_id!, children);
			}
		}
		const list = [...categories.entries()]
			.map(([cid, ts]) => ({
				category: cid ? api.threads.cache.get(cid)! : null,
				threads: ts,
			}))
			.sort((a, b) => {
				// null category comes first
				if (!a.category) return -1;
				if (!b.category) return 1;

				// categories with positions come first
				if (a.category.position === null && b.category.position === null) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (a.category.position === null) return 1;
				if (b.category.position === null) return -1;

				// order by position
				const p = a.category.position! - b.category.position!;
				if (p === 0) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}

				return p;
			});
		setCategories(list);
	});

	// helper to get thread id from the element's data attribute
	const getThreadId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.threadId;

	const handleDragStart = (e: DragEvent) => {
		const id = getThreadId(e);
		if (id) setDragging(id);
	};

	const handleDragEnter = (e: DragEvent) => {
		e.preventDefault();
		setTarget(getThreadId(e) ?? null);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target();
		console.log("handle drop", { fromId, toId });

		if (!fromId || !toId || fromId === toId) {
			setDragging(null);
			setTarget(null);
			return;
		}

		const fromThread = api.threads.cache.get(fromId);
		const toThread = api.threads.cache.get(toId);
		console.log("handle drop threads", { fromThread, toThread });

		if (!fromThread || !toThread) {
			setDragging(null);
			setTarget(null);
			return;
		}

		const fromCategory = categories().find((c) =>
			c.category?.id === fromThread.parent_id ||
			(c.category === null && fromThread.parent_id === null)
		);
		const toCategory = categories().find((c) =>
			c.category?.id === toThread.parent_id ||
			(c.category === null && toThread.parent_id === null)
		);
		console.log("handle drop categories", { fromCategory, toCategory });
		if (!fromCategory || !toCategory) {
			setDragging(null);
			setTarget(null);
			return;
		}

		// remove thread from fromCategory, add to toCategory
		const fromIndex = fromCategory.threads.findIndex((t) => t.id === fromId);
		const toIndex = toCategory.threads.findIndex((t) => t.id === toId);
		const updatedFrom = [...fromCategory.threads];
		const updatedTo = [...toCategory.threads];
		const [moved] = updatedFrom.splice(fromIndex, 1);
		if (fromCategory === toCategory) updatedTo.splice(fromIndex, 1);
		updatedTo.splice(toIndex, 0, moved);
		moved.parent_id = toThread.parent_id;
		console.log("handle drop moved", moved);

		// update positions in toCategory
		for (let i = 0; i < updatedTo.length; i++) {
			if (updatedTo[i].position === null && i > fromIndex && i > toIndex) break;
			console.log(updatedTo[i], i);
			updatedTo[i].position = i;
		}

		const body = updatedTo.map((t) => ({
			id: t.id,
			parent_id: t.parent_id,
			position: t.position,
		}));
		console.log("handle drop body", body);

		api.client.http.PATCH("/api/v1/room/{room_id}/thread", {
			params: { path: { room_id: props.room_id! } },
			body: {
				threads: body,
			},
		});

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

				<For each={categories()}>
					{({ category, threads }) => (
						<>
							<Show when={category}>
								<div class="dim" style="margin-left:8px;margin-top:8px">
									{category!.name}
								</div>
							</Show>
							<For
								each={threads}
								fallback={
									<div class="dim" style="margin-left: 16px">(no threads)</div>
								}
							>
								{(thread, idx) => (
									<li
										data-thread-id={thread.id}
										draggable
										onDragStart={handleDragStart}
										onDragEnter={handleDragEnter}
										onDragOver={handleDragOver}
										onDrop={handleDrop}
										classList={{
											dragging: dragging() === thread.id,
											over: target() === thread.id,
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
													? api.room_members.fetch(
														() => props.room_id!,
														() => s.user_id,
													)
													: () => null;
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
																((voice.rtc?.speaking.get(s.user_id)?.flags ??
																	0) &
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
						</>
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
				unread: props.thread.type !== "Voice" && !!props.thread.is_unread,
			}}
			data-thread-id={props.thread.id}
		>
			{props.thread.name}
		</A>
	);
};
