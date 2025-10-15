import { createSignal } from "solid-js";
import { cyrb53, getColor } from "./colors";
import pfpsUrl from "./assets/pfps.png";

const SIZE = 80;
const layers = [6, 6, 6];

const pfpsImg = new Image();
pfpsImg.src = pfpsUrl;

export const [pfpsLoaded, setPfpsLoaded] = createSignal(false);
pfpsImg.onload = () => {
	setPfpsLoaded(true);
};

const pfpCache = new Map<string, string>();

function LCG(seed: number) {
	let state = seed;
	return function () {
		state = (1103515245 * state + 12345) % 2147483648;
		return state / 2147483648;
	};
}

export async function generatePfp(userId: string): Promise<string> {
	if (!pfpsLoaded()) return "";
	if (pfpCache.has(userId)) {
		return pfpCache.get(userId)!;
	}

	const canvas = new OffscreenCanvas(SIZE, SIZE);
	const ctx = canvas.getContext("2d");
	if (!ctx) {
		return "";
	}

	const rand = LCG(cyrb53(userId));
	const rnd = (n: number) => Math.floor(rand() * n);

	ctx.fillStyle = getColor(userId);
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
