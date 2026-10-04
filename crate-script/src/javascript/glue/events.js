// TODO(?): automatically sync this with ts-sdk/core/events.ts (via tsc --target esnext?)

export class Emitter {
    listeners = new Map();
    on(event, listener) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, new Set());
        }
        this.listeners.get(event).add(listener);
        return () => this.off(event, listener);
    }
    once(event, listener) {
        const wrapper = (data) => {
            this.off(event, wrapper);
            listener(data);
        };
        return this.on(event, wrapper);
    }
    off(event, listener) {
        const set = this.listeners.get(event);
        if (set) {
            set.delete(listener);
        }
    }
    clear(event) {
        if (event) {
            this.listeners.delete(event);
        }
        else {
            this.listeners.clear();
        }
    }
    emit(event, data) {
        const set = this.listeners.get(event);
        if (set) {
            for (const listener of set) {
                try {
                    listener(data);
                }
                catch (err) {
                    console.error(`Error in listener for event "${String(event)}"`, err);
                }
            }
        }
    }
}
