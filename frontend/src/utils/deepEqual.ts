export function deepEqual(a: unknown, b: unknown): boolean {
	if (a === b) {
		return true;
	}

	if (
		typeof a === "object" &&
		a !== null &&
		typeof b === "object" &&
		b !== null
	) {
		if (Array.isArray(a) && Array.isArray(b)) {
			if (a.length !== b.length) return false;
			for (let i = 0; i < a.length; i++) {
				if (!deepEqual(a[i], b[i])) return false;
			}
			return true;
		}
		if (!Array.isArray(a) && !Array.isArray(b)) {
			const keysA = Object.keys(a);
			const keysB = Object.keys(b);
			if (keysA.length !== keysB.length) return false;
			for (const key of keysA) {
				if (!deepEqual(a[key], b[key])) return false;
			}
			return true;
		}
	}
	return false;
}
