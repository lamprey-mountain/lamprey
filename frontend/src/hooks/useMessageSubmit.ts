import { useCtx } from "../context";
import { useChannel } from "../channelctx";
import { useApi, useApi2 } from "../api";

export function useMessageSubmit(channel_id: string) {
	const ctx = useCtx();
	const api = useApi();
	const store = useApi2();
	const channelContext = useChannel();

	return async (text: string, bypassSlowmode?: boolean) => {
		if (!channelContext) return false;
		const [ch, chUpdate] = channelContext;

		if (text.startsWith("/")) {
			await ctx.slashCommands.run(ctx, api, channel_id, text);
			return true;
		}

		const atts = ch.attachments;
		const reply_id = ch.reply_id;
		if (text.length === 0 && atts.length === 0) return false;
		if (!atts.every((i) => i.status === "uploaded")) return false;
		const attachments = atts.map((i) => i.media);

		const channel = api.channels.cache.get(channel_id);
		const messagesService = store?.messages ?? api.messages;

		messagesService.send(channel_id, {
			content: text || null,
			reply_id,
			attachments,
			embeds: [],
		});

		if (channel?.slowmode_message && !bypassSlowmode) {
			const now = new Date();
			const expireAt = new Date(
				now.getTime() + channel.slowmode_message * 1000,
			);
			chUpdate("slowmode_expire_at", expireAt);
		}

		chUpdate("attachments", []);
		chUpdate("reply_id", undefined);
		return true;
	};
}
