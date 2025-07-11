import type { Middleware, ThreadAction } from "../types";
import { handleSubmit } from "../submit.ts";

export const threadSend: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "thread.send") {
		const { thread_id, text } = action as ThreadAction;
		handleSubmit(ctx, thread_id, text, update, api);
	} else {
		next(action);
	}
};
