import { Emitter } from "../core/events";

export class Media {
	// fetch()
	// update()
	// delete()
	// clone()
}

// export type MediaRef = Upload | Media | string | { type: "attachment", ... };
export type MediaRef = Upload | Media | string; // rename to MediaResolvable?

export class MediaManager {
	upload(create: UploadCreate): Upload {
		throw "todo";
	}

	// TODO: make this take something more than just a url
	async import(url: string): Promise<Media> {
		throw "todo";
	}

	// fetch(id)
	// search(query)
}

// maybe move upload to a separate file?
// reuse/port ../upload.ts

export type UploadEvents = {
	fail: Error;
	progress: UploadProgress;
	complete: Media;
	pause: never;
	resume: never;
};

export type UploadProgress = {
	uploadedBytes: number;
	totalBytes: number;
};

export type UploadCreate = File | Blob;

export class Upload extends Emitter<UploadEvents> {
	// media_id: string;
	// pause(): void;
	// resume(): void;
	// abort(): void;
}
