(() => {
    const INTERNAL = Symbol("AbortSignal");
    const ABORT = Symbol("abort");

    function abortError() {
        const error = new Error("The operation was aborted");
        error.name = "AbortError";
        return error;
    }

    function timeoutError() {
        const error = new Error("The operation timed out");
        error.name = "TimeoutError";
        return error;
    }

    function reportError(error) {
        if (typeof setTimeout === "function") {
            setTimeout(() => {
                throw error;
            }, 0);
        }
    }

    class AbortSignal {
        #aborted = false;
        #reason;
        #onabort = null;
        #listeners = new Map();

        constructor(token) {
            if (token !== INTERNAL) {
                throw new TypeError("Illegal constructor");
            }
        }

        static abort(reason) {
            const signal = new AbortSignal(INTERNAL);
            signal[ABORT](reason);
            return signal;
        }

        static timeout(milliseconds) {
            const controller = new AbortController();
            const delay = Number(milliseconds);

            if (typeof setTimeout === "function") {
                setTimeout(() => {
                    controller.abort(timeoutError());
                }, delay);
            }

            return controller.signal;
        }

        static any(signals) {
            const controller = new AbortController();
            const list = Array.from(signals);

            for (const signal of list) {
                if (!(signal instanceof AbortSignal)) {
                    throw new TypeError("expected an AbortSignal");
                }
                if (signal.aborted) {
                    controller.abort(signal.reason);
                    return controller.signal;
                }
            }

            for (const signal of list) {
                signal.addEventListener(
                    "abort",
                    () => {
                        controller.abort(signal.reason);
                    },
                    { once: true },
                );
            }

            return controller.signal;
        }

        get aborted() {
            return this.#aborted;
        }

        get reason() {
            return this.#reason;
        }

        get onabort() {
            return this.#onabort;
        }

        set onabort(value) {
            this.#onabort = typeof value === "function" ? value : null;
        }

        get [Symbol.toStringTag]() {
            return "AbortSignal";
        }

        throwIfAborted() {
            if (this.#aborted) {
                throw this.#reason;
            }
        }

        addEventListener(type, callback, options = {}) {
            if (callback === null || callback === undefined) {
                return;
            }

            const listener =
                typeof callback === "function"
                    ? callback
                    : typeof callback.handleEvent === "function"
                      ? callback
                      : null;
            if (!listener) {
                return;
            }

            const once = typeof options === "boolean" ? false : !!options.once;
            let listeners = this.#listeners.get(type);
            if (!listeners) {
                listeners = [];
                this.#listeners.set(type, listeners);
            }

            if (listeners.some((entry) => entry.callback === listener)) {
                return;
            }

            listeners.push({ callback: listener, once });
        }

        removeEventListener(type, callback) {
            const listeners = this.#listeners.get(type);
            if (!listeners) {
                return;
            }

            const index = listeners.findIndex((entry) => entry.callback === callback);
            if (index !== -1) {
                listeners.splice(index, 1);
            }
        }

        dispatchEvent(event) {
            if (event === null || typeof event !== "object" || typeof event.type !== "string") {
                throw new TypeError("expected an event object");
            }

            this.#dispatch(event);
            return !event.defaultPrevented;
        }

        #dispatch(event) {
            const listeners = this.#listeners.get(event.type);
            if (!listeners || listeners.length === 0) {
                return;
            }

            for (const entry of listeners.slice()) {
                if (entry.once) {
                    this.removeEventListener(event.type, entry.callback);
                }

                try {
                    if (typeof entry.callback === "function") {
                        entry.callback.call(this, event);
                    } else {
                        entry.callback.handleEvent.call(entry.callback, event);
                    }
                } catch (error) {
                    reportError(error);
                }
            }
        }

        [ABORT](reason) {
            if (this.#aborted) {
                return;
            }

            this.#aborted = true;
            this.#reason = reason === undefined ? abortError() : reason;

            const event = {
                type: "abort",
                target: this,
                currentTarget: this,
                bubbles: false,
                cancelable: false,
                defaultPrevented: false,
                timeStamp: Date.now(),
            };

            const onabort = this.#onabort;
            if (onabort) {
                try {
                    onabort.call(this, event);
                } catch (error) {
                    reportError(error);
                }
            }

            this.#dispatch(event);
        }
    }

    class AbortController {
        #signal = new AbortSignal(INTERNAL);

        get signal() {
            return this.#signal;
        }

        get [Symbol.toStringTag]() {
            return "AbortController";
        }

        abort(reason) {
            this.#signal[ABORT](reason);
        }
    }

    Object.assign(globalThis, {
        AbortController,
        AbortSignal,
    });
})();
