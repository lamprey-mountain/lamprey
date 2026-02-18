import type { Middleware } from "../types";

export const categoryMarkRead: Middleware = (
	ctx,
	api,
	_update,
) =>
(next) =>
async (action) => {
	if (action.do === "category.mark_read") {
		const { category_id } = action;
		const category = api.channels.cache.get(category_id);
		if (!category || category.type !== "Category") {
			console.warn("not a category");
			return;
		}

		const childChannels = Array.from(api.channels.cache.values()).filter(
			(c) => c.parent_id === category_id && c.room_id === category.room_id,
		);

		const acks = childChannels
			.map((c) => {
				const version_id = c.last_version_id;
				if (!version_id) return null;
				return { channel_id: c.id, version_id };
			})
			.filter((ack): ack is NonNullable<typeof ack> => ack !== null);

		await api.channels.ackBulk(acks);

		for (const child of childChannels) {
			if (child.last_version_id) {
				const cc = ctx.channel_contexts.get(child.id);
				if (cc) {
					const [_ch, chUpdate] = cc;
					chUpdate("read_marker_id", child.last_version_id);
				}
			}
		}
	} else {
		next(action);
	}
};
