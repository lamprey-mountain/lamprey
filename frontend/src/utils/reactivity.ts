import {
	type Accessor,
	createSignal,
	type Setter,
	type SignalOptions,
} from "solid-js";

export type Signal2<T> = {
	get: Accessor<T>;
	set: Setter<T>;
};

export function createSignal2<T>(
	val: T,
	options?: SignalOptions<T>,
): Signal2<T> {
	const [get, set] = createSignal(val, options);
	return { get, set };
}

// maybe consider using this instead of [get, set], could be cleaner in a lot of places
