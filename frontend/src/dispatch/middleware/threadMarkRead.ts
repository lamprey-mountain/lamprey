import type { Middleware, ThreadAction } from "../types";

export const threadMarkRead: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
async (action) => {
	if (action.do === "thread.mark_read") {
		const { thread_id, version_id, delay, also_local } = action as ThreadAction;
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

		if (also_local) {
			ctx.thread_read_marker_id.set(thread_id, version_id);
		}
		await api.threads.ack(thread_id, undefined, version_id);
	} else {
		next(action);
	}
};
