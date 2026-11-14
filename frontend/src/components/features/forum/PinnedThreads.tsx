import type { Channel } from "sdk";
import { createMemo, createSignal, For, Show } from "solid-js";
import { useChannels } from "@/api";
import { ThreadCard } from "./ThreadCard";

type DropTarget = { id: string; after: boolean };

const reorder = (ids: string[], fromId: string, target: DropTarget) => {
	const fromIndex = ids.indexOf(fromId);
	let toIndex = ids.indexOf(target.id);
	if (fromIndex === -1 || toIndex === -1 || fromId === target.id) return ids;

	if (target.after) toIndex++;
	if (fromIndex < toIndex) toIndex--;

	const next = [...ids];
	const [moved] = next.splice(fromIndex, 1);
	next.splice(toIndex, 0, moved);
	return next;
};

export function PinnedThreads(props: {
	channel: Channel;
	openInSidebar: boolean;
	canManage: boolean;
}) {
	const channels2 = useChannels();
	const [dragging, setDragging] = createSignal<string | null>(null);
	const [target, setTarget] = createSignal<DropTarget | null>(null);

	const pinnedIds = () => props.channel.pinned_thread_ids ?? [];

	const previewIds = createMemo(() => {
		const fromId = dragging();
		const dropTarget = target();
		if (!fromId || !dropTarget) return pinnedIds();
		return reorder(pinnedIds(), fromId, dropTarget);
	});

	const threadId = (e: DragEvent) =>
		(e.currentTarget as HTMLElement).dataset.threadId;

	const handleDragStart = (e: DragEvent) => {
		if (!props.canManage) {
			e.preventDefault();
			return;
		}

		const id = threadId(e);
		if (!id) return;

		setDragging(id);
		const dataTransfer = e.dataTransfer;
		if (dataTransfer) {
			dataTransfer.effectAllowed = "move";
			dataTransfer.setData("text/plain", id);
		}
	};

	const handleDragOver = (e: DragEvent) => {
		if (!props.canManage) return;
		e.preventDefault();

		const id = threadId(e);
		if (!id || id === dragging()) {
			setTarget(null);
			return;
		}

		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const after = e.clientY > rect.top + rect.height / 2;
		if (target()?.id !== id || target()?.after !== after) {
			setTarget({ id, after });
		}
	};

	const handleDrop = (e: DragEvent) => {
		if (!props.canManage) return;
		e.preventDefault();

		const fromId = dragging();
		const dropTarget = target();
		setDragging(null);
		setTarget(null);

		if (!fromId || !dropTarget || fromId === dropTarget.id) return;

		const next = reorder(pinnedIds(), fromId, dropTarget);
		if (next.join(",") === pinnedIds().join(",")) return;

		const previous = channels2.cache.get(props.channel.id);
		if (previous) {
			channels2.cache.set(props.channel.id, {
				...previous,
				pinned_thread_ids: next,
			});
		}

		channels2
			.update(props.channel.id, { pinned_thread_ids: next })
			.catch((err) => {
				console.error("Failed to reorder pinned threads", err);
				if (previous) channels2.cache.set(props.channel.id, previous);
			});
	};

	const handleDragEnd = () => {
		setDragging(null);
		setTarget(null);
	};

	return (
		<Show when={pinnedIds().length > 0}>
			<h3 class="pinned-threads-heading">pinned threads</h3>
			<ul class="pinned-thread-list">
				<For each={previewIds()}>
					{(id) => (
						<PinnedThreadItem
							id={id}
							openInSidebar={props.openInSidebar}
							draggable={props.canManage}
							dragging={dragging() === id}
							dropBefore={target()?.id === id && !target()?.after}
							dropAfter={target()?.id === id && target()?.after === true}
							onDragStart={handleDragStart}
							onDragOver={handleDragOver}
							onDrop={handleDrop}
							onDragEnd={handleDragEnd}
						/>
					)}
				</For>
			</ul>
		</Show>
	);
}

function PinnedThreadItem(props: {
	id: string;
	openInSidebar: boolean;
	draggable: boolean;
	dragging: boolean;
	dropBefore: boolean;
	dropAfter: boolean;
	onDragStart: (e: DragEvent) => void;
	onDragOver: (e: DragEvent) => void;
	onDrop: (e: DragEvent) => void;
	onDragEnd: () => void;
}) {
	const channels2 = useChannels();
	const thread = channels2.use(() => props.id);

	return (
		<li
			data-thread-id={props.id}
			draggable={props.draggable}
			onDragStart={props.onDragStart}
			onDragOver={props.onDragOver}
			onDrop={props.onDrop}
			onDragEnd={props.onDragEnd}
			classList={{
				dragging: props.dragging,
				"drop-before": props.dropBefore,
				"drop-after": props.dropAfter,
			}}
		>
			<Show when={thread()}>
				{(thread) => (
					<ThreadCard thread={thread()} openInSidebar={props.openInSidebar} />
				)}
			</Show>
		</li>
	);
}
