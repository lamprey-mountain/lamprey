export function createObservable<T>(initial: T) {
	let current = initial;
	const listeners = new Set<(val: T) => void>();
	return {
		set(val: T) {
			current = val;
			for (const listener of listeners) {
				listener(val);
			}
		},
		observable(fn: (val: T) => void) {
			listeners.add(fn);
			fn(current);
			return () => {
				listeners.delete(fn);
			};
		},
	};
}
