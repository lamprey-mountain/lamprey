import { Media } from "sdk";
import { VoidProps } from "solid-js";

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
		return m.source.width;
	} else {
		return 0;
	}
};

export const getHeight = (m: Media) => {
	const t = m.source.type;
	if (t === "Video" || t === "Mixed" || t === "Image" || t === "Thumbnail") {
		return m.source.height;
	} else {
		return 0;
	}
};

export const byteFmt = Intl.NumberFormat("en", {
	notation: "compact",
	style: "unit",
	unit: "byte",
	unitDisplay: "narrow",
});
