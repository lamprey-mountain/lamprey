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
