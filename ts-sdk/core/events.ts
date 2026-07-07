export type Events = Record<PropertyKey, unknown>;

export type Unsubscribe = () => void;

export class Emitter<E extends Events> {
	private listeners = new Map<keyof E, Set<(data: any) => void>>();

	on<K extends keyof E>(event: K, listener: (data: E[K]) => void): Unsubscribe {
		if (!this.listeners.has(event)) {
			this.listeners.set(event, new Set());
		}
		this.listeners.get(event)!.add(listener as any);
		return () => this.off(event, listener);
	}

	once<K extends keyof E>(
		event: K,
		listener: (data: E[K]) => void,
	): Unsubscribe {
		const wrapper = (data: E[K]) => {
			this.off(event, wrapper);
			listener(data);
		};
		return this.on(event, wrapper);
	}

	off<K extends keyof E>(event: K, listener: (data: E[K]) => void) {
		const set = this.listeners.get(event);
		if (set) {
			set.delete(listener);
		}
	}

	clear<K extends keyof E>(event?: K) {
		if (event) {
			this.listeners.delete(event);
		} else {
			this.listeners.clear();
		}
	}

	emit<K extends keyof E>(event: K, data: E[K]) {
		const set = this.listeners.get(event);
		if (set) {
			for (const listener of set) {
				try {
					listener(data);
				} catch (err) {
					console.error(`Error in listener for event "${String(event)}"`, err);
				}
			}
		}
	}
}
