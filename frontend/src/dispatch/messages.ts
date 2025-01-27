import { batch as solidBatch } from "solid-js";
import { reconcile, SetStoreFunction } from "solid-js/store";
import { Action, ChatCtx, Data, Slice, TimelineItem } from "../context.ts";
import { TimelineItemT } from "../Messages.tsx";
import { MessageT, Pagination } from "../types.ts";

type RenderTimelineParams = {
	items: Array<TimelineItem>;
	slice: Slice;
	read_marker_id: string | null;
	has_before: boolean;
	has_after: boolean;
};

export function calculateSlice(
	old: Slice | undefined,
	off: number,
	len: number,
	dir: "b" | "f",
): Slice {
	// messages are approx. 32 px high, show 3 pages of messages
	const SLICE_LEN = Math.ceil(globalThis.innerHeight / 32) * 3;

	// scroll a page at a time
	const PAGINATE_LEN = Math.ceil(globalThis.innerHeight / 32);

	if (!old) {
		const end = len;
		const start = Math.max(end - SLICE_LEN, 0);
		return { start, end };
	} else if (dir == "b") {
		const start = Math.max(old.start + off - PAGINATE_LEN, 0);
		const end = Math.min(start + SLICE_LEN, len);
		return { start, end };
	} else {
		const end = Math.min(old.end + off + PAGINATE_LEN, len);
		const start = Math.max(end - SLICE_LEN, 0);
		return { start, end };
	}
}

export function renderTimeline(
	{ items, slice, read_marker_id, has_before, has_after }: RenderTimelineParams,
): Array<TimelineItemT> {
	const rawItems = items.slice(slice.start, slice.end) ?? [];
	const newItems: Array<TimelineItemT> = [];

	if (rawItems.length === 0) throw new Error("no items");

	if (has_before) {
		newItems.push({
			type: "info",
			id: "info",
			header: false,
		});
		newItems.push({
			type: "spacer",
			id: "spacer-top",
		});
	} else {
		newItems.push({
			type: "spacer-mini2",
			id: "spacer-top2",
		});
		newItems.push({
			type: "info",
			id: "info",
			header: true,
		});
	}

	for (let i = 0; i < rawItems.length; i++) {
		const msg = rawItems[i];
		if (msg.type === "hole") continue;
		newItems.push({
			type: "message",
			id: msg.message.nonce ?? msg.message.version_id,
			message: msg.message,
			separate: true,
			is_local: msg.type === "local",
			// separate: shouldSplit(messages[i], messages[i - 1]),
		});
		// if (msg.id - prev.originTs > 1000 * 60 * 5) return true;
		// items.push({
		//   type: "message",
		//   id: messages[i].id,
		//   message: messages[i],
		//   separate: true,
		//   // separate: shouldSplit(messages[i], messages[i - 1]),
		// });
		if (msg.message.id === read_marker_id && i !== rawItems.length - 1) {
			newItems.push({
				type: "unread-marker",
				id: "unread-marker",
			});
		}
	}

	if (has_after) {
		newItems.push({
			type: "spacer",
			id: "spacer-bottom",
		});
	} else {
		newItems.push({
			type: "spacer-mini",
			id: "spacer-bottom-mini",
		});
	}

	return newItems;
}

async function fetchMessages(
	ctx: ChatCtx,
	thread_id: string,
	from: string,
	dir: "b" | "f",
) {
	const { data, error } = await ctx.client.http.GET(
		"/api/v1/thread/{thread_id}/message",
		{
			params: {
				path: { thread_id },
				query: {
					dir,
					from,
					limit: 100,
				},
			},
		},
	);
	if (error) throw error;
	return data;
}

export async function dispatchMessages(
	ctx: ChatCtx,
	update: SetStoreFunction<Data>,
	action: Action,
) {
	switch (action.do) {
		case "paginate": {
			const { dir, thread_id } = action;
			const oldSlice = ctx.data.slices[thread_id] as Slice | undefined;

			// fetch items
			let upd;
			let offset: number = 0;
			if (!oldSlice) {
				const from = "ffffffff-ffff-ffff-ffff-ffffffffffff";
				const batch = await fetchMessages(ctx, thread_id, from, dir);
				const tl: Array<TimelineItem> = batch.items.map((i: MessageT) => ({
					type: "remote" as const,
					message: i,
				}));
				if (batch.has_more) tl.unshift({ type: "hole" });
				upd = () => {
					update("timelines", thread_id, tl);
					update("slices", thread_id, { start: 0, end: tl.length });
					for (const msg of batch.items) {
						update("messages", msg.id, msg);
					}
					offset = batch.items.length;
					// ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
				};
			} else {
				const tl = ctx.data.timelines[thread_id];
				if (tl.length < 2) return; // needs startitem and nextitem
				if (dir === "b") {
					const startItem = tl[oldSlice.start];
					const nextItem = tl[oldSlice.start + 1];
					let batch: Pagination<MessageT> | undefined;
					if (startItem?.type === "hole") {
						const from = nextItem.type === "remote"
							? nextItem.message.id
							: "ffffffff-ffff-ffff-ffff-ffffffffffff";
						batch = await fetchMessages(ctx, thread_id, from, dir);
					}
					upd = () => {
						if (batch) {
							update("timelines", thread_id, (i) =>
								[
									...batch.has_more ? [{ type: "hole" }] : [],
									...batch.items.map((j: MessageT) => ({
										type: "remote",
										message: j,
									})),
									...i.slice(oldSlice.start + 1),
								] as Array<TimelineItem>);
							for (const msg of batch.items) {
								update("messages", msg.id, msg);
							}
							offset = batch.items.length;
						}
					};
				} else {
					const startItem = tl[oldSlice.end - 1];
					const nextItem = tl[oldSlice.end - 2];
					let batch: Pagination<MessageT> | undefined;
					if (startItem.type === "hole") {
						const from = nextItem.type === "remote"
							? nextItem.message.id
							: "00000000-0000-0000-0000-000000000000";
						batch = await fetchMessages(ctx, thread_id, from, dir);
					}

					// PERF: indexOf 115ms
					// PERF: reanchor 95.1ms
					// PERF: getting stuff from store? 362ms
					// PERF: setstore: 808ms
					// PERF: set scroll position: 76.6ms
					upd = () => {
						if (batch) {
							update("timelines", thread_id, (i) =>
								[
									...i.slice(0, oldSlice.end - 1),
									...batch.items.map((j: MessageT) => ({
										type: "remote",
										message: j,
									})),
									...batch.has_more ? [{ type: "hole" }] : [],
								] as Array<TimelineItem>);
							for (const msg of batch.items) {
								update("messages", msg.id, msg);
							}

							offset = batch.items.length;
						}
					};
				}
			}

			solidBatch(() => {
				upd();
				
				const tl = ctx.data.timelines[thread_id];
				const slice = calculateSlice(oldSlice, offset, tl.length, dir);
				update("slices", thread_id, slice);

				const { read_marker_id } = ctx.data.thread_state[thread_id];
				const newItems = renderTimeline({
					items: tl,
					slice,
					read_marker_id,
					has_before: tl.at(0)?.type === "hole",
					has_after: tl.at(-1)?.type === "hole",
				});
				update(
					"thread_state",
					thread_id,
					"timeline",
					(old) => [...reconcile(newItems)(old)],
				);
			});

			return;
		}
	}
}
