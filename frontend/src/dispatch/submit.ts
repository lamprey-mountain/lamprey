import { batch as solidBatch } from "solid-js";
import { ChatCtx, Data } from "../context.ts";
import { SetStoreFunction } from "solid-js/store";
import { uuidv7 } from "uuidv7";
import { MessageT, MessageType } from "../types.ts";
import { Api } from "../api.tsx";

// TODO: implement a retry queue
// TODO: show when messages fail to send
export async function handleSubmit(
	ctx: ChatCtx,
	thread_id: string,
	text: string,
	update: SetStoreFunction<Data>,
	api: Api,
) {
	if (text.startsWith("/")) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const { room_id } = api.threads.cache.get(thread_id)!;
		if (cmd === "thread") {
			const name = text.slice("/thread ".length);
			await ctx.client.http.POST("/api/v1/room/{room_id}/thread", {
				params: { path: { room_id } },
				body: { name },
			});
		} else if (cmd === "archive") {
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					is_closed: true,
				},
			});
		} else if (cmd === "unarchive") {
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					is_closed: false,
				},
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
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: {
					description: description || null,
				},
			});
		} else if (cmd === "name-room") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: { name },
			});
		}
		return;
	}
	const ts = ctx.data.thread_state[thread_id];
	if (text.length === 0 && ts.attachments.length === 0) return false;
	if (!ts.attachments.every((i) => i.status === "uploaded")) return false;
	const attachments = ts.attachments.map((i) => i.media);
	const reply_id = ts.reply_id;
	const nonce = uuidv7();
	ctx.client.http.POST("/api/v1/thread/{thread_id}/message", {
		params: {
			path: { thread_id },
		},
		body: {
			content: text,
			reply_id,
			nonce,
			attachments,
		},
	});
	const localMessage: MessageT = {
		type: MessageType.Default,
		id: nonce,
		thread_id,
		version_id: nonce,
		override_name: null,
		reply_id,
		nonce,
		content: text,
		author: api.users.cache.get("@self")!,
		metadata: null,
		attachments,
		is_pinned: false,
		ordering: 0,
	};
	solidBatch(() => {
		update(
			"timelines",
			thread_id,
			(i) => [...i, { type: "local" as const, message: localMessage }],
		);
		// TODO: is this necessary?
		// update("messages", msg.id, msg);
		update("thread_state", thread_id, "reply_id", null);
		update("thread_state", thread_id, "attachments", []);
		ctx.dispatch({ do: "thread.autoscroll", thread_id });
	});
}
