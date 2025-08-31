import type { Media, MediaTrack } from "sdk";
import type { ParentProps, VoidProps } from "solid-js";
import { flags } from "../flags";
import { CDN_URL } from "../App.tsx";

export type MediaProps = VoidProps<{ media: Media }>;

export function formatTime(time: number): string {
	const t = Math.floor(time);
	const seconds = t % 60;
	const minutes = Math.floor(t / 60) % 60;
	const hours = Math.floor(t / 3600);
	if (hours) {
		return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")
			}`;
	} else {
		return `${minutes}:${seconds.toString().padStart(2, "0")}`;
	}
}

/** in seconds */
export const getDuration = (m: Media) => {
	const t = m.source.type;
	if (t === "Audio" || t === "Mixed" || t === "Video") {
		return (m.source.duration ?? 0) / 1000;
	} else {
		return 0;
	}
};

export const getWidth = (m: Media) => {
	const t = m.source.type;
	if (t === "Video" || t === "Mixed" || t === "Image" || t === "Thumbnail") {
		return m.source.width ?? 0;
	} else {
		return 0;
	}
};

export const getHeight = (m: Media) => {
	const t = m.source.type;
	if (t === "Video" || t === "Mixed" || t === "Image" || t === "Thumbnail") {
		return m.source.height ?? 0;
	} else {
		return 0;
	}
};

/** get the cdn url for a piece of media */
export const getUrl = (t: Media) => {
	return `${CDN_URL}/media/${t}`
};

/** get the cdn url for the thumbnail for a piece of media */
export const getThumb = (media: Media, size?: number) => {
	if (size) {
		return `${CDN_URL}/thumb/${media.id}?size=${size}`
	} else {
		return `${CDN_URL}/thumb/${media.id}`
	}
};

export const byteFmt = Intl.NumberFormat("en", {
	notation: "compact",
	style: "unit",
	unit: "byte",
	unitDisplay: "narrow",
});

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
