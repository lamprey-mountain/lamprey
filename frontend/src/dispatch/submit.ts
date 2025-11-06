import type { ChatCtx, Data } from "../context.ts";
import type { SetStoreFunction } from "solid-js/store";
import type { Api } from "../api.tsx";
import { handleSlashCommand } from "../slash-commands.ts";

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
		await handleSlashCommand(ctx, api, thread_id, text);
		return;
	}

	const atts = ctx.channel_attachments.get(atts_thread_id ?? thread_id) ?? [];
	const reply_id = ctx.channel_reply_id.get(thread_id);
	if (text.length === 0 && atts.length === 0) return false;
	if (!atts.every((i) => i.status === "uploaded")) return false;
	const attachments = atts.map((i) => i.media);

	const channel = api.channels.cache.get(thread_id);

	api.messages.send(thread_id, {
		content: text || null,
		reply_id,
		attachments,
		embeds: [],
	});

	if (channel?.slowmode_message) {
		const now = new Date();
		const expireAt = new Date(now.getTime() + channel.slowmode_message * 1000);
		ctx.channel_slowmode_expire_at.set(thread_id, expireAt);
	}

	ctx.channel_attachments.delete(atts_thread_id ?? thread_id);
	ctx.channel_reply_id.delete(thread_id);
}
