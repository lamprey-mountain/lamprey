import { createContext, onCleanup, onMount, useContext } from "solid-js";
import { createUpload, type Media } from "sdk";
import type { Attachment, ChatCtx } from "../context";
import type { ReactiveMap } from "@solid-primitives/map";
import { useModals } from "./modal";

export type UploadController = {
	init: (local_id: string, thread_id: string, file: File) => void;
	pause: (local_id: string) => void;
	resume: (local_id: string) => void;
	cancel: (local_id: string, thread_id: string) => void;
};

const UploadsContext = createContext<UploadController>();

export const UploadsProvider = (props: { ctx: ChatCtx; children: any }) => {
	const [, modalCtl] = useModals();

	// Track pending uploads by media_id for async processing
	const pendingUploads = new Map<
		string,
		{ local_id: string; thread_id: string }
	>();

	// Listen for MediaProcessed and MediaUpdate events
	onMount(() => {
		const handleMediaProcessed = (media: Media) => {
			// Find the attachment that was waiting for this media to be processed
			const pending = pendingUploads.get(media.id);
			if (pending) {
				const [ch, chUpdate] = props.ctx.channel_contexts.get(
					pending.thread_id,
				)!;
				const atts = ch.attachments;
				const idx = atts.findIndex((a) => a.local_id === pending.local_id);
				if (idx !== -1) {
					const att: Attachment = {
						status: "uploaded",
						media,
						local_id: pending.local_id,
						spoiler: atts[idx].spoiler,
					};
					chUpdate("attachments", atts.toSpliced(idx, 1, att));
				}
				pendingUploads.delete(media.id);
			}
		};

		const handleMediaUpdate = (media: Media) => {
			// Update all attachments that reference this media
			for (const [thread_id, ctx] of props.ctx.channel_contexts.entries()) {
				const [ch, chUpdate] = ctx;
				const atts = ch.attachments;
				const idx = atts.findIndex((a) =>
					a.status === "uploaded" && a.media.id === media.id
				);
				if (idx !== -1) {
					const att = atts[idx];
					const updatedAtt: Attachment = {
						...att,
						media,
					};
					chUpdate("attachments", atts.toSpliced(idx, 1, updatedAtt));
				}
			}
		};

		props.ctx.events.on("sync", ([msg]) => {
			if (msg.type === "MediaProcessed") {
				handleMediaProcessed(msg.media);
			} else if (msg.type === "MediaUpdate") {
				handleMediaUpdate(msg.media);
			}
		});
	});

	const init = (local_id: string, thread_id: string, file: File) => {
		const [ch, chUpdate] = props.ctx.channel_contexts.get(thread_id)!;

		// Add initial attachment
		chUpdate("attachments", [...ch.attachments, {
			status: "uploading",
			file,
			local_id,
			progress: 0,
			paused: false,
			filename: file.name,
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
				modalCtl.alert(error.message);
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
			// Track this upload for async processing
			pendingUploads.set(upload.media_id, { local_id, thread_id });
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
		// Remove from pending uploads
		pendingUploads.delete(upload.media_id);
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
