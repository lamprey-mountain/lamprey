import type { ChatCtx, Data } from "../context.ts";
import type { SetStoreFunction } from "solid-js/store";
import type { Api } from "../api.tsx";

// TODO: implement a retry queue
// TODO: show when messages fail to send
export async function handleSubmit(
	ctx: ChatCtx,
	thread_id: string,
	text: string,
	_update: SetStoreFunction<Data>,
	api: Api,
	atts_thread_id?: string,
) {
	if (text.startsWith("/")) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const { room_id } = api.channels.cache.get(thread_id)!;
		if (cmd === "thread") {
			if (!room_id) return;
			const name = text.slice("/thread ".length);
			await ctx.client.http.POST("/api/v1/room/{room_id}/thread", {
				params: { path: { room_id } },
				body: { name },
			});
		} else if (cmd === "archive") {
			await ctx.client.http.PUT("/api/v1/thread/{thread_id}/archive", {
				params: { path: { thread_id } },
			});
		} else if (cmd === "unarchive") {
			await ctx.client.http.DELETE("/api/v1/thread/{thread_id}/archive", {
				params: { path: { thread_id } },
			});
		} else if (cmd === "remove") {
			await ctx.client.http.PUT("/api/v1/thread/{thread_id}/remove", {
				params: { path: { thread_id } },
			});
		} else if (cmd === "unremove") {
			await ctx.client.http.DELETE("/api/v1/thread/{thread_id}/remove", {
				params: { path: { thread_id } },
			});
		} else if (cmd === "desc") {
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					description: description || null,
				},
			});
		} else if (cmd === "name") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: { name },
			});
		} else if (cmd === "desc-room") {
			if (!room_id) return;
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: {
					description: description || null,
				},
			});
		} else if (cmd === "name-room") {
			if (!room_id) return;
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: { name },
			});
		}
		return;
	}
	const atts = ctx.thread_attachments.get(atts_thread_id ?? thread_id) ?? [];
	const reply_id = ctx.thread_reply_id.get(thread_id);
	if (text.length === 0 && atts.length === 0) return false;
	if (!atts.every((i) => i.status === "uploaded")) return false;
	const attachments = atts.map((i) => i.media);
	api.messages.send(thread_id, {
		content: text || null,
		reply_id,
		attachments,
		embeds: [],
	});
	ctx.thread_attachments.delete(atts_thread_id ?? thread_id);
	ctx.thread_reply_id.delete(thread_id);
}
