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
		const [ch, chUpdate] = ctx.channel_contexts.get(thread_id)!;
		const atts = ch.attachments;
		chUpdate("attachments", [...atts, {
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
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att: Attachment = {
					status: "uploading",
					file,
					local_id,
					progress,
					paused: false,
				};
				chUpdate("attachments", atts.toSpliced(idx, 1, att));
			},
			onFail(error) {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				chUpdate("attachments", atts.toSpliced(idx, 1));
				ctx.dispatch({ do: "modal.alert", text: error.message });
			},
			onComplete(media) {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att: Attachment = {
					status: "uploaded",
					media,
					local_id,
					file,
				};
				chUpdate("attachments", atts.toSpliced(idx, 1, att));
			},
			onPause() {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: true,
				};
				chUpdate("attachments", atts.toSpliced(idx, 1, att));
			},
			onResume() {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: false,
				};
				chUpdate("attachments", atts.toSpliced(idx, 1, att));
			},
		});
		ctx.uploads.set(local_id, up);
	} else {
		next(action);
	}
};
