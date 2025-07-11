import type { Middleware } from "../types";

export const log: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	console.log("dispatch", action, ctx.data);
	next(action);
};
