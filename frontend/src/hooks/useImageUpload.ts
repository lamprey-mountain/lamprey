import { createUpload, type Media } from "sdk";
import { createSignal } from "solid-js";
import { useApi } from "@/api";

export type ImageUploadOptions = {
	onComplete: (media: Media) => void;
	onError?: (error: Error) => void;
};

export function useImageUpload(opts: ImageUploadOptions) {
	const api = useApi();
	const [progress, setProgress] = createSignal<number | null>(null);

	const uploading = () => progress() !== null;

	const upload = (file: File) => {
		setProgress(0);
		createUpload({
			client: api.client,
			file,
			onComplete(media) {
				opts.onComplete(media);
				setProgress(null);
			},
			onFail(error) {
				opts.onError?.(error);
				setProgress(null);
			},
			onPause() {},
			onResume() {},
			onProgress(value) {
				// ignore progress from a finished/cancelled upload
				setProgress((prev) => (prev === null ? null : value));
			},
		}).catch((error: unknown) => {
			console.error("image upload failed", error);
			opts.onError?.(
				error instanceof Error ? error : new Error(String(error)),
			);
			setProgress(null);
		});
	};

	const onInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
		const file = e.currentTarget.files?.[0];
		if (file) upload(file);
	};

	const openPicker = (input: HTMLInputElement | undefined) => {
		if (uploading()) return;
		input?.click();
	};

	return { upload, onInput, openPicker, progress, uploading };
}
