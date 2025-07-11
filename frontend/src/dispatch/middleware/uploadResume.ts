import type { Middleware, UploadAction } from "../types";

export const uploadResume: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "upload.resume") {
		const { local_id } = action as UploadAction;
		ctx.uploads.get(local_id)?.resume();
	} else {
		next(action);
	}
};
