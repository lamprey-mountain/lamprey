import type { Middleware, UploadAction } from "../types";
import { createUpload } from "sdk";
import type { Attachment } from "../../context";

export const uploadInit: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
async (action) => {
	if (action.do === "upload.init") {
		const { local_id, thread_id, file } = action as UploadAction;
		const atts = ctx.channel_attachments.get(thread_id) ?? [];
		ctx.channel_attachments.set(thread_id, [...atts, {
			status: "uploading",
			file,
			local_id,
			progress: 0,
			paused: false,
		}]);
		const up = await createUpload({
			file,
			client: ctx.client,
			onProgress(progress) {
				const atts = ctx.channel_attachments.get(thread_id)!;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att: Attachment = {
					status: "uploading",
					file,
					local_id,
					progress,
					paused: false,
				};
				ctx.channel_attachments.set(thread_id, atts.toSpliced(idx, 1, att));
			},
			onFail(error) {
				const atts = ctx.channel_attachments.get(thread_id)!;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				ctx.channel_attachments.set(thread_id, atts.toSpliced(idx, 1));
				ctx.dispatch({ do: "modal.alert", text: error.message });
			},
			onComplete(media) {
				const atts = ctx.channel_attachments.get(thread_id)!;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att: Attachment = {
					status: "uploaded",
					media,
					local_id,
					file,
				};
				ctx.channel_attachments.set(thread_id, atts.toSpliced(idx, 1, att));
			},
			onPause() {
				const atts = ctx.channel_attachments.get(thread_id)!;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: true,
				};
				ctx.channel_attachments.set(thread_id, atts.toSpliced(idx, 1, att));
			},
			onResume() {
				const atts = ctx.channel_attachments.get(thread_id)!;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: false,
				};
				ctx.channel_attachments.set(thread_id, atts.toSpliced(idx, 1, att));
			},
		});
		ctx.uploads.set(local_id, up);
	} else {
		next(action);
	}
};
