export { default as Color } from "colorjs.io";

export function getColor(id: string) {
	const last = id.at(-1);
	if (!last) return "#ffffff";
	switch (parseInt(last, 16) % 8) {
		case 0:
			return "oklch(74.03% 0.1759 13.16)"; // red
		case 1:
			return "oklch(85.53% 0.1395 130.14)"; // green
		case 2:
			return "oklch(85.39% 0.1187 92.43)"; // yellow
		case 3:
			return "oklch(79.29% 0.1636 255.6)"; // blue
		case 4:
			return "oklch(80.6% 0.15 299.2)"; // magenta
		case 5:
			return "oklch(80.21% 0.1086 199.72)"; // cyan
		case 6:
			return "oklch(80.7% 0.1273 50.56)"; // orange
		case 7:
			return "oklch(80% 0.128 168)"; // teal
	}
}

export const colors = {
	fg100: "oklch(var(--color-fg1))",
	fg200: "oklch(var(--color-fg2))",
	fg300: "oklch(var(--color-fg3))",
	fg400: "oklch(var(--color-fg4))",
	fg500: "oklch(var(--color-fg5))",
	fg600: "oklch(var(--color-fg6))",
};

export type ColorSpace = "oklch" | "srgb";

export function oklchToRgb(
	l: number,
	c: number,
	cosH: number,
	sinH: number,
): [number, number, number] {
	const a = c * cosH;
	const b = c * sinH;

	// OKLab -> LMS
	const l_ = l + 0.3963377774 * a + 0.2158037573 * b;
	const m_ = l - 0.1055613458 * a - 0.0638541728 * b;
	const s_ = l - 0.0894841775 * a - 1.291485548 * b;

	const l3 = l_ * l_ * l_;
	const m3 = m_ * m_ * m_;
	const s3 = s_ * s_ * s_;

	// LMS -> linear sRGB
	let r = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
	let g = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
	let bl = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.707614701 * s3;

	// linear -> gamma-encoded sRGB
	const toSrgb = (v: number) => {
		v = Math.min(1, Math.max(0, v));
		return v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055;
	};
	r = toSrgb(r);
	g = toSrgb(g);
	bl = toSrgb(bl);

	return [Math.round(r * 255), Math.round(g * 255), Math.round(bl * 255)];
}
