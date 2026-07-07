import type { Lamprey } from "ts-sdk";

const protoCache = new WeakMap<Lamprey, WeakMap<Function, object>>();

export type Resource<T, P> = {
	wrap(client: Lamprey, data: T): T & P;
};

type WithThis<P, Self> = {
	[K in keyof P]: P[K] extends (...args: infer A) => infer R
		? (this: Self, ...args: A) => R
		: P[K];
};

export function defineResource<T, P extends object>(
	makeProto: (client: Lamprey) => WithThis<P, T & P>,
): Resource<T, P> {
	return {
		wrap(client, data) {
			let clientCache = protoCache.get(client);
			if (!clientCache) protoCache.set(client, (clientCache = new WeakMap()));
			let proto = clientCache.get(makeProto) as P | undefined;
			if (!proto) clientCache.set(makeProto, (proto = makeProto(client) as P));
			return Object.setPrototypeOf(data, proto) as T & P;
		},
	};
}
