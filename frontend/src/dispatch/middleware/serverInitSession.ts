import type { Middleware, ServerAction } from "../types";

export const serverInitSession: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "server.init_session") {
		api.tempCreateSession();
	} else {
		next(action);
	}
};
