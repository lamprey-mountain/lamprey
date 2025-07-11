import type { Middleware, UploadAction } from "../types";

export const uploadCancel: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "upload.cancel") {
		const { local_id, thread_id } = action as UploadAction;
		const upload = ctx.uploads.get(local_id);
		if (!upload) return;
		upload.abort();
		ctx.uploads.delete(local_id);
		const atts = ctx.thread_attachments.get(thread_id)!;
		const idx = atts.findIndex((i) => i.local_id === local_id)!;
		if (idx !== -1) {
			ctx.thread_attachments.set(thread_id, atts.toSpliced(idx, 1));
		}
	} else {
		next(action);
	}
};
