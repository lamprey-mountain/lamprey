import { getTimestampFromUUID, Message } from "sdk";

export function createWeaklyMemoized<T extends object, U>(fn: (_: T) => U): (_: T) => U {
	const cache = new WeakMap();
	return (t: T) => {
		const cached = cache.get(t);
		if (cached) return cached;
		const ran = fn(t);
		cache.set(t, ran);
		return ran;
	};
}

export const getMsgTs = createWeaklyMemoized((m: Message) =>
	getTimestampFromUUID(m.id)
);
