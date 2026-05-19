import { createUpload, type Media, type MessageSync } from "sdk";
import { createContext, onMount, type ParentProps, useContext } from "solid-js";
import type { Attachment, ChatCtx } from "@/types/chat";
import { useModals } from "./modal";

export type UploadController = {
	init: (local_id: string, thread_id: string, file: File) => void;
	pause: (local_id: string) => void;
	resume: (local_id: string) => void;
	cancel: (local_id: string, thread_id: string) => void;
};

const UploadsContext = createContext<UploadController>();

export const UploadsProvider = (props: ParentProps<{ ctx: ChatCtx }>) => {
	const [, modalCtl] = useModals();

	// Listen for MediaUpdate events
	onMount(() => {
		const handleMediaUpdate = (media: Media) => {
			// Update all attachments that reference this media
			for (const [_thread_id, ctx] of props.ctx.channel_contexts.entries()) {
				const [ch, chUpdate] = ctx;
				const atts = ch.attachments;
				const idx = atts.findIndex(
					(a) => a.status === "uploaded" && a.media.id === media.id,
				);
				if (idx !== -1) {
					const att = atts[idx];
					const updatedAtt: Attachment = {
						...att,
					} as Attachment;
					chUpdate("attachments", [
						...atts.slice(0, idx),
						updatedAtt,
						...atts.slice(idx + 1),
					]);
				}
			}
		};

		props.ctx.events.on("sync", ([msg]) => {
			if (msg.type === "MediaUpdate") {
				handleMediaUpdate(msg.media);
			}
		});
	});

	const init = (local_id: string, thread_id: string, file: File) => {
		const [ch, chUpdate] = props.ctx.channel_contexts.get(thread_id)!;

		// Add initial attachment
		chUpdate("attachments", [
			...ch.attachments,
			{
				status: "uploading",
				file,
				local_id,
				progress: 0,
				paused: false,
				filename: file.name,
			},
		]);

		// Create upload
		createUpload({
			file,
			client: props.ctx.client,
			onProgress(progress) {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att: Attachment = {
					...(atts[idx] as any),
					status: "uploading",
					progress,
				};
				chUpdate("attachments", [
					...atts.slice(0, idx),
					att,
					...atts.slice(idx + 1),
				]);
			},
			onFail(error) {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				chUpdate("attachments", [
					...atts.slice(0, idx),
					...atts.slice(idx + 1),
				]);
				// Replace dispatch with modal controller
				modalCtl.alert(error.message);
			},
			onComplete(media) {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const oldAtt = atts[idx];
				const att: Attachment = {
					status: "uploaded",
					media,
					local_id: oldAtt.local_id,
					spoiler: oldAtt.spoiler,
				};
				chUpdate("attachments", [
					...atts.slice(0, idx),
					att,
					...atts.slice(idx + 1),
				]);
			},
			onPause() {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: true,
				};
				chUpdate("attachments", [
					...atts.slice(0, idx),
					att,
					...atts.slice(idx + 1),
				]);
			},
			onResume() {
				const atts = ch.attachments;
				const idx = atts.findIndex((i) => i.local_id === local_id);
				if (idx === -1) return;
				const att = {
					...atts[idx],
					paused: false,
				};
				chUpdate("attachments", [
					...atts.slice(0, idx),
					att,
					...atts.slice(idx + 1),
				]);
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
			chUpdate("attachments", [...atts.slice(0, idx), ...atts.slice(idx + 1)]);
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
