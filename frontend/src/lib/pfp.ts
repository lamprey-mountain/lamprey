import { createSignal } from "solid-js";
import pfpsUrl from "@/assets/pfps.png";
import { getColor } from "@/lib/colors";
import { cyrb53, LCG } from "@/lib/rng";

const SIZE = 80;
const layers = [6, 6, 6];

const pfpsImg = new Image();
pfpsImg.src = pfpsUrl;

export const [pfpsLoaded, setPfpsLoaded] = createSignal(false);
pfpsImg.onload = () => {
	setPfpsLoaded(true);
};

const pfpCache = new Map<string, string>();

export async function generatePfp(userId: string): Promise<string> {
	if (!pfpsLoaded()) return "";
	const cached = pfpCache.get(userId);
	if (cached) {
		return cached;
	}

	const canvas = new OffscreenCanvas(SIZE, SIZE);
	const ctx = canvas.getContext("2d");
	if (!ctx) {
		return "";
	}

	const rand = LCG(cyrb53(userId));
	const rnd = (n: number) => Math.floor(rand() * n);

	const color = getColor(userId);
	if (color) {
		ctx.fillStyle = color;
	}
	ctx.fillRect(0, 0, SIZE, SIZE);

	const PADDING = 0;
	for (let i = 0; i < layers.length; i++) {
		const numOptions = layers[i];
		ctx.drawImage(
			pfpsImg,
			SIZE * rnd(numOptions),
			SIZE * i,
			SIZE,
			SIZE,
			PADDING,
			PADDING,
			SIZE - PADDING * 2,
			SIZE - PADDING * 2,
		);
	}

	const blob = await canvas.convertToBlob();
	const dataUrl = URL.createObjectURL(blob);
	pfpCache.set(userId, dataUrl);
	return dataUrl;
}

export async function generateRoomIcon(roomId: string): Promise<string> {
	if (!pfpsLoaded()) return "";
	const cached = pfpCache.get(roomId);
	if (cached) {
		return cached;
	}

	const canvas = new OffscreenCanvas(SIZE, SIZE);
	const ctx = canvas.getContext("2d");
	if (!ctx) {
		return "";
	}

	const rand = LCG(cyrb53(roomId));
	const rnd = (n: number) => Math.floor(rand() * n);

	const color = getColor(roomId);
	if (color) {
		ctx.fillStyle = color;
	}
	ctx.fillRect(0, 0, SIZE, SIZE);

	// TODO: layers for room icons
	// const PADDING = 0;
	// for (let i = 0; i < layers.length; i++) {
	// 	const numOptions = layers[i];
	// 	ctx.drawImage(
	// 		pfpsImg,
	// 		SIZE * rnd(numOptions),
	// 		SIZE * i,
	// 		SIZE,
	// 		SIZE,
	// 		PADDING,
	// 		PADDING,
	// 		SIZE - PADDING * 2,
	// 		SIZE - PADDING * 2,
	// 	);
	// }

	const blob = await canvas.convertToBlob();
	const dataUrl = URL.createObjectURL(blob);
	pfpCache.set(roomId, dataUrl);
	return dataUrl;
}
