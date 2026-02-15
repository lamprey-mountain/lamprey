import type { Middleware, ThreadAction } from "../types";

export const threadMarkRead: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
async (action) => {
	if (action.do === "thread.mark_read") {
		const { thread_id, version_id, delay, also_local } = action;
		// NOTE: may need separate timeouts per thread
		let ackGraceTimeout: number | undefined;
		let ackDebounceTimeout: number | undefined;
		clearTimeout(ackGraceTimeout);
		clearTimeout(ackDebounceTimeout);
		if (delay) {
			ackGraceTimeout = setTimeout(() => {
				ackDebounceTimeout = setTimeout(() => {
					ctx.dispatch({ ...action, delay: false });
				}, 800);
			}, 200);
			return;
		}

		const cc = ctx.channel_contexts.get(thread_id);

		if (cc) {
			const [_ch, chUpdate] = cc;
			if (also_local) {
				chUpdate("read_marker_id", version_id);
			}
			await api.channels.ack(thread_id, undefined, version_id);
		} else {
			const c = api.channels.cache.get(thread_id);
			if (!c) throw new Error("could not find channel " + thread_id);
			await api.channels.ack(thread_id, undefined, c.last_version_id!);
		}
	} else {
		next(action);
	}
};
