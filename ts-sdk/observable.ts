export type Observable<T> = {
	set(val: T): void;
	get(): T;
	observable: Observer<T>;
};

export type Observer<T> = (subscriber: (val: T) => void) => () => void;

export function createObservable<T>(
	initial: T,
	onListenerChange?: (size: number) => void,
): Observable<T> {
	let current = initial;
	const listeners = new Set<(val: T) => void>();
	return {
		set(val: T) {
			current = val;
			for (const listener of listeners) {
				listener(val);
			}
		},
		get() {
			return current;
		},
		observable(fn: (val: T) => void) {
			listeners.add(fn);
			fn(current);
			onListenerChange?.(listeners.size);
			return () => {
				listeners.delete(fn);
				onListenerChange?.(listeners.size);
			};
		},
	};
}

export type ObservableMap<K, V> = {
	read: {
		has(key: K): boolean;
		get(key: K): V;
		watch(key: K): Observer<V>;
		entries(): Iterator<[K, V]>;
		watchEntries(): Observer<Array<[K, Observer<V>]>>;
	};
	write: {
		set(key: K, value: V): void;
		delete(key: K): void;
	};
};

export function createObservableMap<K, V>(empty: V): ObservableMap<K, V> {
	const entries = new Map<K, Observable<V>>();
	const listing = createObservable<Array<[K, Observer<V>]>>([]);

	const cleanup = (key: K, size: number) => {
		if (size !== 0) return;
		if (entries.get(key) === empty) entries.delete(key);
	};

	const init = (key: K) => {
		const o = entries.get(key);
		if (o) return o;
		const no = createObservable(empty, (size) => cleanup(key, size));
		entries.set(key, no);
		return no;
	};

	const has = (key: K) => {
		const o = entries.get(key);
		if (!o) return false;
		return o.get() !== empty;
	};
	
	return {
		read: {
			has,
			get(key: K) {
				return init(key).get();
			},
			watch(key: K) {
				return init(key).observable;
			},
			entries() {
				return entries
					.entries()
					.filter(([_k, o]) => o.get() !== empty)
					.map(([k, o]) => [k, o.get()]);
			},
			watchEntries() {
				return listing.observable;
			},
		},
		write: {
			set(key: K, value: V) {
				const exists = has(key);
				init(key).set(value);
				if (!exists) {
					const arr = entries
						.entries()
						.filter(([_k, o]) => o.get() !== empty)
						.map(([k, o]) => [k, o.observable])
						.toArray();
					listing.set(arr as Array<[K, Observer<V>]>);
				}
			},
			delete(key: K) {
				const o = entries.get(key);
				if (!o) return;
				if (o === empty) return;
				o.set(empty);
				const arr = entries
					.entries()
					.filter(([_k, o]) => o.get() !== empty)
					.map(([k, o]) => [k, o.observable])
					.toArray();
				listing.set(arr as Array<[K, Observer<V>]>);
			},
		},
	};
}
