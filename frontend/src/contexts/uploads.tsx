import { createContext, useContext } from "solid-js";
import { createUpload } from "sdk";
import type { Attachment, ChatCtx } from "../context";
import type { ReactiveMap } from "@solid-primitives/map";

export type UploadController = {
	init: (local_id: string, thread_id: string, file: File) => void;
	pause: (local_id: string) => void;
	resume: (local_id: string) => void;
	cancel: (local_id: string, thread_id: string) => void;
};

const UploadsContext = createContext<UploadController>();

export const UploadsProvider = (props: { ctx: ChatCtx; children: any }) => {
	const init = (local_id: string, thread_id: string, file: File) => {
		const [ch, chUpdate] = props.ctx.channel_contexts.get(thread_id)!;

		// Add initial attachment
		chUpdate("attachments", [...ch.attachments, {
			status: "uploading",
			file,
			local_id,
			progress: 0,
			paused: false,
		}]);

		// Create upload
		createUpload({
			file,
			client: props.ctx.client,
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
				// Replace dispatch with modal controller
				props.ctx.modalCtl.alert(error.message);
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
		}).then((upload) => {
			props.ctx.uploads.set(local_id, upload);
		});
	};

	const pause = (local_id: string) => {
		props.ctx.uploads.get(local_id)?.pause();
	};

	const resume = (local_id: string) => {
		props.ctx.uploads.get(local_id)?.resume();
	};

	const cancel = (local_id: string, thread_id: string) => {
		const upload = props.ctx.uploads.get(local_id);
		if (!upload) return;
		upload.abort();
		const [ch, chUpdate] = props.ctx.channel_contexts.get(thread_id)!;
		props.ctx.uploads.delete(local_id);
		const atts = ch.attachments;
		const idx = atts.findIndex((i) => i.local_id === local_id)!;
		if (idx !== -1) {
			chUpdate("attachments", atts.toSpliced(idx, 1));
		}
	};

	const controller: UploadController = {
		init,
		pause,
		resume,
		cancel,
	};

	return (
		<UploadsContext.Provider value={controller}>
			{props.children}
		</UploadsContext.Provider>
	);
};

export const useUploads = () => {
	const ctx = useContext(UploadsContext);
	if (!ctx) {
		throw new Error("useUploads must be used within an UploadsProvider");
	}
	return ctx;
};
