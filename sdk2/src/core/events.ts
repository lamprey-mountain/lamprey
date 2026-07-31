export type Events = Record<PropertyKey, (...args: any[]) => void>;

export type Unsubscribe = () => void;

export class Emitter<E extends Events> {
	private listeners = new Map<keyof E, Set<(...args: any[]) => void>>();

	public on<K extends keyof E>(event: K, listener: E[K]): Unsubscribe {
		if (!this.listeners.has(event)) {
			this.listeners.set(event, new Set());
		}
		this.listeners.get(event)!.add(listener as any);
		return () => this.off(event, listener);
	}

	public once<K extends keyof E>(event: K, listener: E[K]): Unsubscribe {
		const wrapper = ((...args: Parameters<E[K]>) => {
			this.off(event, wrapper);
			listener(...args);
		}) as unknown as E[K];

		return this.on(event, wrapper);
	}

	public off<K extends keyof E>(event: K, listener: E[K]) {
		const set = this.listeners.get(event);
		if (set) {
			set.delete(listener);
		}
	}

	public clear<K extends keyof E>(event?: K) {
		if (event) {
			this.listeners.delete(event);
		} else {
			this.listeners.clear();
		}
	}

	public emit<K extends keyof E>(event: K, ...args: Parameters<E[K]>) {
		const set = this.listeners.get(event);
		if (set) {
			for (const listener of set) {
				try {
					listener(...args);
				} catch (err) {
					console.error(`Error in listener for event "${String(event)}"`, err);
				}
			}
		}
	}
}
