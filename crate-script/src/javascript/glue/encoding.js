(() => {
    class TextEncoderStream extends TransformStream {
        constructor() {
            super(
                {
                    transform(chunk, controller) {
                        controller.enqueue(new TextEncoder().encode(chunk));
                    },
                },
                { highWaterMark: 0 },
                { highWaterMark: 0 },
            );
        }

        get encoding() {
            return "utf-8";
        }
    }

    class TextDecoderStream extends TransformStream {
        #encoding;
        #fatal;
        #ignoreBOM;

        constructor(label = "utf-8", options = {}) {
            const decoder = new TextDecoder(label, options);

            super(
                {
                    transform(chunk, controller) {
                        try {
                            const text = decoder.decode(chunk, { stream: true });
                            if (text) {
                                controller.enqueue(text);
                            }
                        } catch (error) {
                            controller.error(error);
                            throw error;
                        }
                    },
                    flush(controller) {
                        try {
                            const text = decoder.decode();
                            if (text) {
                                controller.enqueue(text);
                            }
                        } catch (error) {
                            controller.error(error);
                            throw error;
                        }
                    },
                },
                { highWaterMark: 0 },
                { highWaterMark: 0 },
            );

            this.#encoding = decoder.encoding;
            this.#fatal = decoder.fatal;
            this.#ignoreBOM = decoder.ignoreBOM;
        }

        get encoding() {
            return this.#encoding;
        }

        get fatal() {
            return this.#fatal;
        }

        get ignoreBOM() {
            return this.#ignoreBOM;
        }
    }

    Object.assign(globalThis, {
        TextEncoderStream,
        TextDecoderStream,
    });
})();
