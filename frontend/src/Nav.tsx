import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { A, useNavigate, useParams } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Thread } from "sdk";
import { flags } from "./flags.ts";
import { useVoice } from "./voice-provider.tsx";
import { useConfig } from "./config.tsx";
import { Avatar, AvatarWithStatus, getColor, ThreadIcon } from "./User.tsx";
import { getThumbFromId } from "./media/util.tsx";

export const ThreadNav = (props: { room_id?: string }) => {
	const config = useConfig();
	const api = useApi();
	const [voice] = useVoice();
	const params = useParams();
	const nav = useNavigate();

	// track drag ids
	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<
		{ id: string; after: boolean } | null
	>(
		null,
	);

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

	const previewedCategories = createMemo(() => {
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;
		const cats = categories();

		if (!fromId || !toId || fromId === toId) return cats;

		const fromThread = api.threads.cache.get(fromId);
		const toThread = api.threads.cache.get(toId);
		if (!fromThread || !toThread) return cats;

		const newCategories = cats.map((c) => ({
			category: c.category,
			threads: [...c.threads],
		}));

		const fromCat = newCategories.find(
			(c) => (c.category?.id ?? null) === fromThread.parent_id,
		);
		if (!fromCat) return cats;
		const fromIndex = fromCat.threads.findIndex((t) => t.id === fromId);
		if (fromIndex === -1) return cats;

		const [moved] = fromCat.threads.splice(fromIndex, 1);

		if (toThread.type === "Category") {
			const toCat = newCategories.find((c) => c.category?.id === toId);
			if (!toCat) return cats;
			if (after) toCat.threads.push(moved);
			else toCat.threads.unshift(moved);
		} else {
			const toCat = newCategories.find(
				(c) => (c.category?.id ?? null) === toThread.parent_id,
			);
			if (!toCat) return cats;
			let toIndex = toCat.threads.findIndex((t) => t.id === toId);
			if (toIndex === -1) return cats;
			if (after) toIndex++;
			toCat.threads.splice(toIndex, 0, moved);
		}

		return newCategories;
	});

	// helper to get thread id from the element's data attribute
	const getThreadId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.threadId;

	const handleDragStart = (e: DragEvent) => {
		const id = getThreadId(e);
		if (id) setDragging(id);
	};

	const handleDragOver = (e: DragEvent) => {
		e.preventDefault();
		const id = getThreadId(e);
		if (!id || id === dragging()) {
			return;
		}
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		e.preventDefault();
		const fromId = dragging();
		const toId = target()?.id;
		const after = target()?.after;

		setDragging(null);
		setTarget(null);

		if (!fromId || !toId || fromId === toId) return;

		const fromThread = api.threads.cache.get(fromId);
		const toThread = api.threads.cache.get(toId);
		if (!fromThread || !toThread) return;

		const fromCategory = categories().find(
			(c) => (c.category?.id ?? null) === fromThread.parent_id,
		);
		if (!fromCategory) return;

		const fromIndex = fromCategory.threads.findIndex((t) => t.id === fromId);
		if (fromIndex === -1) return;

		let toCategory;
		let toIndex;
		let newParentId;

		if (toThread.type === "Category") {
			toCategory = categories().find((c) => c.category?.id === toId);
			if (!toCategory) return;
			toIndex = after ? toCategory.threads.length : 0;
			newParentId = toId;
		} else {
			toCategory = categories().find(
				(c) => (c.category?.id ?? null) === toThread.parent_id,
			);
			if (!toCategory) return;
			toIndex = toCategory.threads.findIndex((t) => t.id === toId);
			if (toIndex === -1) return;
			if (after) toIndex++;
			newParentId = toThread.parent_id;
		}

		const reordered = [...toCategory.threads];
		if (fromCategory === toCategory) {
			if (fromIndex < toIndex) toIndex--;
			const [moved] = reordered.splice(fromIndex, 1);
			reordered.splice(toIndex, 0, moved);
		} else {
			reordered.splice(toIndex, 0, fromThread);
		}

		if (
			fromCategory === toCategory &&
			JSON.stringify(fromCategory.threads.map((t) => t.id)) ===
				JSON.stringify(reordered.map((t) => t.id))
		) {
			return;
		}

		const body = reordered.map((t, i) => ({
			id: t.id,
			parent_id: newParentId,
			position: i,
		}));

		if (fromCategory !== toCategory) {
			const sourceBody = fromCategory.threads
				.filter((t) => t.id !== fromId)
				.map((t, i) => ({
					id: t.id,
					parent_id: fromThread.parent_id,
					position: i,
				}));
			body.push(...sourceBody);
		}

		api.client.http.PATCH("/api/v1/room/{room_id}/thread", {
			params: { path: { room_id: props.room_id! } },
			body: {
				threads: body,
			},
		});
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

				<For each={previewedCategories()}>
					{({ category, threads }) => (
						<>
							<Show when={category}>
								<div
									class="dim"
									style="margin-left:8px;margin-top:8px"
									data-thread-id={category!.id}
									draggable="true"
									onDragStart={handleDragStart}
									onDragOver={handleDragOver}
									onDrop={handleDrop}
									onDragEnd={() => {
										setDragging(null);
										setTarget(null);
									}}
									onClick={[nav, `/thread/${category!.id}`]}
									classList={{
										dragging: dragging() === category!.id,
									}}
								>
									{category!.name}
								</div>
							</Show>
							<For
								each={threads}
								fallback={
									<div class="dim" style="margin-left: 16px">(no threads)</div>
								}
							>
								{(thread) => (
									<li
										data-thread-id={thread.id}
										draggable="true"
										onDragStart={handleDragStart}
										onDragOver={handleDragOver}
										onDrop={handleDrop}
										onDragEnd={() => {
											setDragging(null);
											setTarget(null);
										}}
										classList={{
											dragging: dragging() === thread.id,
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
														<Avatar user={user()} />
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
	const api = useApi();
	const otherUser = createMemo(() => {
		if (props.thread.type === "Dm") {
			const selfId = api.users.cache.get("@self")!.id;
			return props.thread.recipients.find((i) => i.id !== selfId);
		}
		return undefined;
	});

	const name = () => {
		if (props.thread.type === "Dm") {
			return otherUser()?.name ?? "dm";
		}

		return props.thread.name;
	};

	return (
		<A
			href={`/thread/${props.thread.id}`}
			class="menu-thread"
			classList={{
				unread: props.thread.type !== "Voice" && !!props.thread.is_unread,
			}}
			data-thread-id={props.thread.id}
		>
			<Switch>
				<Match when={props.thread.type === "Dm" && otherUser()}>
					<AvatarWithStatus user={otherUser()} />
				</Match>
				<Match when={props.thread.type === "Gdm"}>
					<ThreadIcon id={props.thread.id} icon={props.thread.icon} />
				</Match>
			</Switch>
			<div>
				<div
					style={{
						"text-overflow": "ellipsis",
						overflow: "hidden",
						"white-space": "nowrap",
					}}
				>
					{name()}
				</div>
				<Show when={otherUser()?.status.text}>
					{(t) => <div class="dim">{t()}</div>}
				</Show>
			</div>
		</A>
	);
};
