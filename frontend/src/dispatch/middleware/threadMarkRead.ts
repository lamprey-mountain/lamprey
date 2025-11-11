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

		const [_ch, chUpdate] = ctx.channel_contexts.get(thread_id)!;
		if (also_local) {
			chUpdate("read_marker_id", version_id);
		}
		await api.channels.ack(thread_id, undefined, version_id);
	} else {
		next(action);
	}
};
