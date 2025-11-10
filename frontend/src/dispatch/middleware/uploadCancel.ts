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
		const [ch, chUpdate] = ctx.channel_contexts.get(thread_id)!;
		ctx.uploads.delete(local_id);
		const atts = ch.attachments;
		const idx = atts.findIndex((i) => i.local_id === local_id)!;
		if (idx !== -1) {
			chUpdate("attachments", atts.toSpliced(idx, 1));
		}
	} else {
		next(action);
	}
};
