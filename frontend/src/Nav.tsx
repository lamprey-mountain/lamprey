import { createEffect, createSignal, For, onMount, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { A } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Thread } from "sdk";
import { flags } from "./flags.ts";

export const ChatNav = (props: { room_id?: string }) => {
	const api = useApi();

	// track drag indices
	const [dragging, setDragging] = createSignal<number | null>(null);
	const [target, setTarget] = createSignal<number | null>(null);

	// local list of threads for this room
	const [list, setList] = createSignal<Thread[]>([]);

	if (!props.room_id) {
		api.dms.list();
	}

	// update list when room changes
	createEffect(() => {
		const threads = [...api.threads.cache.values()]
			.filter((t) =>
				(props.room_id ? t.room_id === props.room_id : t.room_id === null) &&
				!t.deleted_at
			);
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
				<header style="background: #eef1;padding:8px">header</header>
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
							}}
						>
							<ItemThread thread={thread} />
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
				unread: props.thread.is_unread,
			}}
			data-thread-id={props.thread.id}
		>
			{props.thread.name}
		</A>
	);
};
