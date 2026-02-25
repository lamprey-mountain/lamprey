import type { Media } from "sdk";
import type { ParentProps, VoidProps } from "solid-js";
import { useConfig } from "../config";

export type MediaProps = VoidProps<{ media: Media }>;

export function formatTime(time: number): string {
	const t = Math.floor(time);
	const seconds = t % 60;
	const minutes = Math.floor(t / 60) % 60;
	const hours = Math.floor(t / 3600);
	if (hours) {
		return `${hours}:${minutes.toString().padStart(2, "0")}:${
			seconds.toString().padStart(2, "0")
		}`;
	} else {
		return `${minutes}:${seconds.toString().padStart(2, "0")}`;
	}
}

/** in seconds */
export const getDuration = (m: Media) => {
	const metadata = m.metadata as any;
	if (metadata.type === "Audio" || metadata.type === "Video") {
		return metadata.duration ?? 0;
	} else {
		return 0;
	}
};

export const getWidth = (m: Media) => {
	const metadata = m.metadata as any;
	if (metadata.type === "Video" || metadata.type === "Image") {
		return metadata.width ?? 0;
	} else {
		return 0;
	}
};

export const getHeight = (m: Media) => {
	const metadata = m.metadata as any;
	if (metadata.type === "Video" || metadata.type === "Image") {
		return metadata.height ?? 0;
	} else {
		return 0;
	}
};

/** get the cdn url for a piece of media */
export const getUrl = (t: Media) => {
	const config = useConfig();
	const url = new URL(`/media/${t.id}`, config.cdn_url);
	return url.href;
};

/** get the cdn url for the thumbnail for a piece of media */
export const getThumb = (media: Media, size?: number) => {
	return getThumbFromId(media.id, size);
};

/** get the cdn url for the thumbnail for a piece of media from its id */
export const getThumbFromId = (
	media_id: string,
	size?: number,
	animate?: boolean,
) => {
	const config = useConfig();
	const url = new URL(`${config.cdn_url}/thumb/${media_id}`);
	if (size) url.searchParams.set("size", size.toString());
	if (animate !== undefined) {
		url.searchParams.set("animate", animate.toString());
	}
	return url.toString();
};

/** get the cdn url for the thumbnail for a custom emoji from its id */
export const getThumbFromEmojiId = (emoji_id: string, size?: number) => {
	const config = useConfig();
	const url = new URL(`/emoji/${emoji_id}`, config.cdn_url);
	if (size) url.searchParams.set("size", size.toString());
	return url.href;
};

/** get the cdn url for an emoji */
export const getEmojiUrl = (id: string) => {
	const config = useConfig();
	const url = new URL(`/emoji/${id}`, config.cdn_url);
	return url.href;
};

export function formatBytes(bytes: number): string {
	const units = [" bytes", "KiB", "MiB", "GiB", "TiB"];
	let size = bytes;
	let unitIndex = 0;

	while (size >= 1024 && unitIndex < units.length - 1) {
		size /= 1024;
		unitIndex++;
	}

	return size.toFixed(2) + units[unitIndex];
}

export type MediaLoadingState =
	| "stalled" // data isn't loading
	| "empty" // no data is loaded
	| "loading" // attempting to load data
	| "ready"; // media is ready to play

export const parseRanges = (b: TimeRanges) =>
	Array(b.length).fill(0).map((_, idx) => ({
		start: b.start(idx),
		end: b.end(idx),
	}));

type ResizeProps = {
	height: number;
	width: number;
	ratio?: number;
};
type LoaderProps = { loaded: boolean };

export const Resize = (props: ParentProps<ResizeProps>) => {
	return (
		<div
			class="resize"
			style={{
				"--height": `${props.height}px`,
				"--width": `${props.width}px`,
				"--aspect-ratio": props.ratio ?? `${props.width}/${props.height}`,
			}}
		>
			{props.children}
		</div>
	);
};

export const Loader = (props: VoidProps<LoaderProps>) => {
	return (
		<div
			class="media-loader"
			classList={{ loaded: props.loaded }}
			role="status"
			aria-label="loading"
			aria-hidden={props.loaded}
		>
			loading
		</div>
	);
};
