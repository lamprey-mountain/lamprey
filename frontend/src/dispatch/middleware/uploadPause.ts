import type { Middleware, UploadAction } from "../types";

export const uploadPause: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "upload.pause") {
		const { local_id } = action as UploadAction;
		ctx.uploads.get(local_id)?.pause();
	} else {
		next(action);
	}
};
