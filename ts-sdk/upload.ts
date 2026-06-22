import type { Client } from "./client.ts";
import type { Media } from "./types.ts";

export type UploadOptions = {
	file: File;
	client: Client;
	onProgress: (progress: number) => void;
	onFail: (error: Error) => void;
	onComplete: (media: Media) => void;
	onPause: () => void;
	onResume: () => void;
};

export type Upload = {
	media_id: string;
	pause(): void;
	resume(): void;
	abort(): void;
};

export async function createUpload(opts: UploadOptions): Promise<Upload> {
	const { data, error } = await opts.client.http.POST("/api/v1/media", {
		body: {
			filename: opts.file.name,
			size: opts.file.size,
		},
	});

	if (error) {
		opts.onFail(error);
		throw new Error(error);
	}

	const { upload_url, media_id } = data;
	if (!upload_url) {
		const err = new Error("missing upload_url in media response");
		opts.onFail(err);
		throw err;
	}

	let offset = 0;
	let currentOffset = 0;
	let xhr: XMLHttpRequest;

	async function resumeUpload() {
		// make sure to cancel the currently in flight upload, in case resume is called multiple times
		xhr?.abort();

		const token = opts.client.opts.token;
		if (!token) {
			opts.onFail(new Error("missing token"));
			return;
		}
		if (!upload_url) {
			opts.onFail(new Error("missing upload_url"));
			return;
		}

		const res = await fetch(upload_url, {
			method: "HEAD",
			headers: {
				authorization: `Bearer ${token}`,
			},
		});
		if (res.ok) {
			const rawOffset = res.headers.get("upload-offset");
			if (rawOffset) {
				offset = parseInt(rawOffset, 10);
				currentOffset = offset;
			}
			attemptUpload();
		} else {
			opts.onFail(
				new Error(
					`upload probe failed: ${(await res.text()) ?? res.statusText}`,
				),
			);
		}
	}

	function attemptUpload() {
		xhr = new XMLHttpRequest();

		xhr.upload.onprogress = (ev) => {
			offset = ev.loaded + currentOffset;
			opts.onProgress(offset / opts.file.size);
		};

		xhr.onload = async () => {
			if (xhr.status === 204) {
				const { data, error } = await opts.client.http.PUT(
					"/api/v1/media/{media_id}/done",
					{
						params: { path: { media_id } },
						body: { async: false },
					},
				);
				if (error) throw error;
				if (data) opts.onComplete(data);
			} else {
				opts.onFail(new Error(`upload failed: ${xhr.responseText}`));
			}
		};

		xhr.onabort = () => {
			console.log("upload manually aborted");
		};

		xhr.onerror = () => {
			console.log("upload failed, retrying in 1s...");
			setTimeout(resumeUpload, 1000);
		};

		const token = opts.client.opts.token;
		if (!token) {
			opts.onFail(new Error("missing token"));
			return;
		}
		if (!upload_url) {
			opts.onFail(new Error("missing upload_url"));
			return;
		}

		xhr.open("PATCH", upload_url);
		xhr.setRequestHeader("authorization", `Bearer ${token}`);
		xhr.setRequestHeader("upload-offset", offset.toString());
		xhr.send(opts.file.slice(offset));
	}

	function pause() {
		xhr?.abort();
		opts.onPause();
	}

	let started = false;
	function resume() {
		// save a roundtrip
		if (started) {
			resumeUpload();
		} else {
			attemptUpload();
			started = true;
		}

		opts.onResume();
	}

	async function abort() {
		xhr?.abort();
		await opts.client.http.DELETE("/api/v1/media/{media_id}", {
			params: {
				path: { media_id },
			},
		});
	}

	resume();
	return { media_id, pause, resume, abort };
}
