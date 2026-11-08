// a compact implementation of the core web streams api
//
// this intentionally focuses on the default (non-BYOB) controllers and
// readers. it is written in javascript so it can use the engine's promise and
// async iterator implementations directly.
(() => {
    function makeError(name, message) {
        const error = new Error(message);
        error.name = name;
        return error;
    }

    function makePromise() {
        let resolve;
        let reject;
        const promise = new Promise((res, rej) => {
            resolve = res;
            reject = rej;
        });
        return { promise, resolve, reject };
    }

    function highWaterMark(strategy, fallback) {
        if (!strategy || strategy.highWaterMark === undefined) {
            return fallback;
        }
        return Number(strategy.highWaterMark);
    }

    function chunkSize(strategy, chunk) {
        if (strategy && typeof strategy.size === "function") {
            const size = Number(strategy.size(chunk));
            if (Number.isNaN(size)) {
                throw makeError("TypeError", "queuing strategy size returned NaN");
            }
            return size;
        }
        return 1;
    }

    function abortError(reason) {
        if (reason instanceof Error) {
            return reason;
        }
        if (reason === undefined) {
            return makeError("AbortError", "The stream was aborted");
        }
        return reason;
    }

    function toBytesView(view) {
        if (!ArrayBuffer.isView(view)) {
            throw makeError("TypeError", "expected an ArrayBufferView");
        }
        return new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
    }

    class ReadableStreamDefaultController {
        #enqueue;
        #closeStream;
        #error;
        #desiredSize;

        constructor(enqueue, closeStream, error, desiredSize) {
            this.#enqueue = enqueue;
            this.#closeStream = closeStream;
            this.#error = error;
            this.#desiredSize = desiredSize;
        }

        get desiredSize() {
            return this.#desiredSize();
        }

        enqueue(chunk) {
            this.#enqueue(chunk);
        }

        close() {
            this.#closeStream();
        }

        error(error) {
            this.#error(error);
        }
    }

    class ReadableStreamDefaultReader {
        #read;
        #cancel;
        #release;
        #closed;

        constructor(read, cancel, release, closed) {
            this.#read = read;
            this.#cancel = cancel;
            this.#release = release;
            this.#closed = closed;
        }

        get closed() {
            return this.#closed;
        }

        read() {
            return this.#read();
        }

        cancel(reason) {
            return this.#cancel(reason);
        }

        releaseLock() {
            this.#release();
        }
    }

    class ReadableStreamBYOBRequest {
        #request;

        constructor(request) {
            this.#request = request;
        }

        get view() {
            return this.#request.view;
        }

        respond(bytesWritten) {
            this.#request.respond(bytesWritten);
        }

        respondWithNewView(view) {
            this.#request.respondWithNewView(view);
        }
    }

    class ReadableStreamBYOBReader {
        #read;
        #cancel;
        #release;
        #closed;

        constructor(read, cancel, release, closed) {
            this.#read = read;
            this.#cancel = cancel;
            this.#release = release;
            this.#closed = closed;
        }

        get closed() {
            return this.#closed;
        }

        read(view) {
            return this.#read(view);
        }

        cancel(reason) {
            return this.#cancel(reason);
        }

        releaseLock() {
            this.#release();
        }
    }

    class ReadableByteStreamController {
        #enqueue;
        #closeStream;
        #error;
        #desiredSize;
        #getByobRequest;

        constructor(enqueue, closeStream, error, desiredSize, getByobRequest) {
            this.#enqueue = enqueue;
            this.#closeStream = closeStream;
            this.#error = error;
            this.#desiredSize = desiredSize;
            this.#getByobRequest = getByobRequest;
        }

        get byobRequest() {
            return this.#getByobRequest();
        }

        get desiredSize() {
            return this.#desiredSize();
        }

        enqueue(chunk) {
            this.#enqueue(chunk);
        }

        close() {
            this.#closeStream();
        }

        error(error) {
            this.#error(error);
        }
    }

    class ReadableStream {
        #source;
        #strategy;
        #highWaterMark;
        #state = "readable";
        #storedError;
        #queue = [];
        #queueTotalSize = 0;
        #reader = null;
        #readerClosed = null;
        #readRequests = [];
        #byobRequests = [];
        #byobRequest = null;
        #bytes = false;
        #closeRequested = false;
        #pulling = false;
        #pullAgain = false;
        #controller;
        #started;

        constructor(source = {}, strategy = {}) {
            this.#source = source || {};
            this.#strategy = strategy || {};
            this.#highWaterMark = highWaterMark(this.#strategy, 1);
            this.#bytes = this.#source.type === "bytes";
            this.#controller = this.#bytes
                ? new ReadableByteStreamController(
                      (chunk) => this.#enqueue(chunk),
                      () => this.#close(),
                      (error) => this.#error(error),
                      () => this.#highWaterMark - this.#queueTotalSize,
                      () => this.#byobRequest,
                  )
                : new ReadableStreamDefaultController(
                      (chunk) => this.#enqueue(chunk),
                      () => this.#close(),
                      (error) => this.#error(error),
                      () => this.#highWaterMark - this.#queueTotalSize,
                  );

            let startResult;
            try {
                startResult =
                    typeof this.#source.start === "function"
                        ? this.#source.start(this.#controller)
                        : undefined;
            } catch (error) {
                startResult = Promise.reject(error);
            }

            this.#started = Promise.resolve(startResult).catch((error) => {
                this.#error(error);
            });
            this.#callPullIfNeeded();
        }

        static from(iterable) {
            const iterator =
                typeof iterable[Symbol.asyncIterator] === "function"
                    ? iterable[Symbol.asyncIterator]()
                    : iterable[Symbol.iterator]();

            return new ReadableStream({
                async pull(controller) {
                    const result = await iterator.next();
                    if (result.done) {
                        controller.close();
                        return;
                    }
                    controller.enqueue(result.value);
                },
                cancel(reason) {
                    if (typeof iterator.return === "function") {
                        return iterator.return(reason);
                    }
                },
            });
        }

        get locked() {
            return this.#reader !== null;
        }

        cancel(reason) {
            return this.#cancel(reason);
        }

        getReader(options = {}) {
            if (this.#reader) {
                throw makeError("TypeError", "stream is already locked");
            }

            const mode = options && options.mode;
            if (mode !== undefined && mode !== "byob") {
                throw makeError("TypeError", "invalid reader mode");
            }
            if (mode === "byob" && !this.#bytes) {
                throw makeError("TypeError", "BYOB readers require a byte stream");
            }

            this.#readerClosed = makePromise();
            const reader =
                mode === "byob"
                    ? new ReadableStreamBYOBReader(
                          (view) => this.#readBYOB(view),
                          (reason) => this.#cancel(reason),
                          () => this.#releaseReader(),
                          this.#readerClosed.promise,
                      )
                    : new ReadableStreamDefaultReader(
                          () => this.#read(),
                          (reason) => this.#cancel(reason),
                          () => this.#releaseReader(),
                          this.#readerClosed.promise,
                      );
            this.#reader = reader;
            return reader;
        }

        pipeTo(destination, options = {}) {
            const preventAbort = !!options.preventAbort;
            const preventCancel = !!options.preventCancel;
            const preventClose = !!options.preventClose;

            return (async () => {
                const reader = this.getReader();
                const writer = destination.getWriter();

                try {
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) {
                            if (!preventClose) {
                                await writer.close();
                            }
                            return;
                        }
                        await writer.write(result.value);
                    }
                } catch (error) {
                    if (!preventAbort) {
                        try {
                            await writer.abort(error);
                        } catch {}
                    }
                    if (!preventCancel) {
                        try {
                            await reader.cancel(error);
                        } catch {}
                    }
                    throw error;
                } finally {
                    reader.releaseLock();
                    writer.releaseLock();
                }
            })();
        }

        pipeThrough(transform, options = {}) {
            const promise = this.pipeTo(transform.writable, options);
            promise.catch(() => {});
            return transform.readable;
        }

        tee() {
            const reader = this.getReader();
            const branches = [0, 1].map(() => {
                let controller;
                const stream = new ReadableStream({
                    start(value) {
                        controller = value;
                    },
                });
                return { stream, controller };
            });

            (async () => {
                try {
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) {
                            for (const branch of branches) {
                                branch.controller.close();
                            }
                            return;
                        }
                        for (const branch of branches) {
                            branch.controller.enqueue(result.value);
                        }
                    }
                } catch (error) {
                    for (const branch of branches) {
                        branch.controller.error(error);
                    }
                } finally {
                    reader.releaseLock();
                }
            })();

            return branches.map((branch) => branch.stream);
        }

        values(options = {}) {
            const reader = this.getReader();
            const preventCancel = !!options.preventCancel;
            return {
                next() {
                    return reader.read();
                },
                return(value) {
                    const result = preventCancel
                        ? Promise.resolve({ value, done: true })
                        : reader.cancel(value).then(() => ({ value, done: true }));
                    reader.releaseLock();
                    return result;
                },
                [Symbol.asyncIterator]() {
                    return this;
                },
            };
        }

        [Symbol.asyncIterator](options = {}) {
            return this.values(options);
        }

        #read() {
            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }

            if (this.#state === "closed" && this.#queue.length === 0) {
                return Promise.resolve({ value: undefined, done: true });
            }

            return new Promise((resolve, reject) => {
                if (this.#queue.length !== 0) {
                    const entry = this.#queue.shift();
                    this.#queueTotalSize -= entry.size;
                    resolve({ value: entry.chunk, done: false });
                    this.#closeIfNeeded();
                    this.#callPullIfNeeded();
                    return;
                }

                this.#readRequests.push({ resolve, reject });
                this.#callPullIfNeeded();
            });
        }

        #readBYOB(view) {
            const bytes = toBytesView(view);
            if (bytes.byteLength === 0) {
                return Promise.reject(
                    makeError("TypeError", "BYOB read view must not be empty"),
                );
            }

            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }

            if (this.#state === "closed" && this.#queue.length === 0) {
                return Promise.resolve({
                    value: new Uint8Array(bytes.buffer, bytes.byteOffset, 0),
                    done: true,
                });
            }

            const filled = this.#fillFromQueue(bytes);
            if (filled > 0) {
                const value = new Uint8Array(bytes.buffer, bytes.byteOffset, filled);
                this.#closeIfNeeded();
                this.#callPullIfNeeded();
                return Promise.resolve({ value, done: false });
            }

            if (this.#state === "closed") {
                return Promise.resolve({
                    value: new Uint8Array(bytes.buffer, bytes.byteOffset, 0),
                    done: true,
                });
            }

            return new Promise((resolve, reject) => {
                this.#byobRequests.push({ view: bytes, resolve, reject });
                this.#updateByobRequest();
                this.#callPullIfNeeded();
            });
        }

        #enqueue(chunk) {
            if (this.#state !== "readable" || this.#closeRequested) {
                throw makeError("TypeError", "cannot enqueue into a closed stream");
            }

            if (this.#bytes) {
                if (!ArrayBuffer.isView(chunk)) {
                    throw makeError("TypeError", "byte streams require an ArrayBufferView");
                }
                this.#enqueueBytes(toBytesView(chunk).slice());
                return;
            }

            const size = chunkSize(this.#strategy, chunk);
            if (this.#readRequests.length !== 0) {
                this.#readRequests.shift().resolve({ value: chunk, done: false });
            } else {
                this.#queue.push({ chunk, size });
                this.#queueTotalSize += size;
            }

            this.#callPullIfNeeded();
        }

        #enqueueBytes(bytes) {
            let remaining = bytes;

            while (remaining.byteLength > 0 && this.#byobRequests.length !== 0) {
                const request = this.#byobRequests[0];
                const take = Math.min(request.view.byteLength, remaining.byteLength);
                request.view.set(remaining.subarray(0, take), 0);
                this.#byobRequests.shift();
                this.#updateByobRequest();
                request.resolve({
                    value: new Uint8Array(request.view.buffer, request.view.byteOffset, take),
                    done: false,
                });
                remaining = remaining.subarray(take);
            }

            if (remaining.byteLength !== 0) {
                this.#queue.push({ chunk: remaining, size: remaining.byteLength });
                this.#queueTotalSize += remaining.byteLength;
            }

            while (this.#readRequests.length !== 0 && this.#queue.length !== 0) {
                const entry = this.#queue.shift();
                this.#queueTotalSize -= entry.size;
                this.#readRequests.shift().resolve({ value: entry.chunk, done: false });
            }

            this.#updateByobRequest();
            this.#callPullIfNeeded();
        }

        #fillFromQueue(view) {
            let filled = 0;

            while (filled < view.byteLength && this.#queue.length !== 0) {
                const entry = this.#queue[0];
                const chunk = entry.chunk;
                const take = Math.min(chunk.byteLength, view.byteLength - filled);
                view.set(chunk.subarray(0, take), filled);
                filled += take;

                if (take === chunk.byteLength) {
                    this.#queue.shift();
                    this.#queueTotalSize -= entry.size;
                } else {
                    entry.chunk = chunk.subarray(take);
                    entry.size = chunk.byteLength - take;
                    this.#queueTotalSize -= take;
                }
            }

            return filled;
        }

        #updateByobRequest() {
            this.#byobRequest =
                this.#byobRequests.length !== 0
                    ? this.#makeByobRequest(this.#byobRequests[0])
                    : null;
        }

        #makeByobRequest(request) {
            return {
                get view() {
                    return request.view;
                },
                respond: (bytesWritten) => {
                    this.#respondByob(request, bytesWritten);
                },
                respondWithNewView: (view) => {
                    this.#respondByob(request, undefined, view);
                },
            };
        }

        #respondByob(request, bytesWritten, newView) {
            if (this.#byobRequests[0] !== request) {
                throw makeError("TypeError", "BYOB request is no longer active");
            }

            if (newView !== undefined) {
                const value = toBytesView(newView);
                this.#byobRequests.shift();
                this.#updateByobRequest();
                request.resolve({ value, done: false });
            } else {
                if (
                    !Number.isInteger(bytesWritten) ||
                    bytesWritten < 0 ||
                    bytesWritten > request.view.byteLength
                ) {
                    throw makeError("RangeError", "invalid bytesWritten value");
                }

                const value = new Uint8Array(
                    request.view.buffer,
                    request.view.byteOffset,
                    bytesWritten,
                );
                this.#byobRequests.shift();
                this.#updateByobRequest();
                request.resolve({ value, done: false });
            }

            this.#callPullIfNeeded();
        }

        #close() {
            if (this.#closeRequested || this.#state !== "readable") {
                return;
            }
            this.#closeRequested = true;
            this.#closeIfNeeded();
        }

        #closeIfNeeded() {
            if (!this.#closeRequested || this.#state !== "readable") {
                return;
            }

            if (this.#bytes) {
                while (this.#queue.length !== 0 && this.#byobRequests.length !== 0) {
                    const request = this.#byobRequests[0];
                    const filled = this.#fillFromQueue(request.view);
                    if (filled === 0) {
                        break;
                    }
                    this.#byobRequests.shift();
                    request.resolve({
                        value: new Uint8Array(
                            request.view.buffer,
                            request.view.byteOffset,
                            filled,
                        ),
                        done: false,
                    });
                }
                this.#updateByobRequest();
            }

            if (this.#queue.length !== 0) {
                return;
            }

            this.#state = "closed";
            for (const request of this.#readRequests) {
                request.resolve({ value: undefined, done: true });
            }
            this.#readRequests = [];

            for (const request of this.#byobRequests) {
                request.resolve({
                    value: new Uint8Array(request.view.buffer, request.view.byteOffset, 0),
                    done: true,
                });
            }
            this.#byobRequests = [];
            this.#updateByobRequest();

            if (this.#readerClosed) {
                this.#readerClosed.resolve();
            }
        }

        #error(error) {
            if (this.#state === "closed" || this.#state === "errored") {
                return;
            }

            this.#state = "errored";
            this.#storedError = error;
            this.#queue = [];
            this.#queueTotalSize = 0;

            for (const request of this.#readRequests) {
                request.reject(error);
            }
            this.#readRequests = [];

            for (const request of this.#byobRequests) {
                request.reject(error);
            }
            this.#byobRequests = [];
            this.#updateByobRequest();

            if (this.#readerClosed) {
                this.#readerClosed.reject(error);
            }
        }

        #cancel(reason) {
            if (this.#state === "closed") {
                return Promise.resolve();
            }
            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }

            this.#state = "closed";
            this.#queue = [];
            this.#queueTotalSize = 0;

            for (const request of this.#readRequests) {
                request.resolve({ value: undefined, done: true });
            }
            this.#readRequests = [];

            for (const request of this.#byobRequests) {
                request.resolve({
                    value: new Uint8Array(request.view.buffer, request.view.byteOffset, 0),
                    done: true,
                });
            }
            this.#byobRequests = [];
            this.#updateByobRequest();

            if (this.#readerClosed) {
                this.#readerClosed.resolve();
            }

            return Promise.resolve(
                typeof this.#source.cancel === "function"
                    ? this.#source.cancel(reason)
                    : undefined,
            ).then(() => undefined);
        }

        #releaseReader() {
            if (!this.#reader) {
                return;
            }

            const error = makeError(
                "TypeError",
                "reader released with pending read requests",
            );

            if (this.#readRequests.length !== 0) {
                for (const request of this.#readRequests) {
                    request.reject(error);
                }
                this.#readRequests = [];
                if (this.#readerClosed) {
                    this.#readerClosed.reject(error);
                }
            }

            if (this.#byobRequests.length !== 0) {
                for (const request of this.#byobRequests) {
                    request.reject(error);
                }
                this.#byobRequests = [];
                if (this.#readerClosed) {
                    this.#readerClosed.reject(error);
                }
            }

            this.#updateByobRequest();
            this.#reader = null;
            this.#readerClosed = null;
        }

        #callPullIfNeeded() {
            if (this.#state !== "readable" || this.#closeRequested) {
                return;
            }
            if (this.#pulling) {
                this.#pullAgain = true;
                return;
            }

            const hasPendingRead =
                this.#readRequests.length !== 0 || this.#byobRequests.length !== 0;
            if (!hasPendingRead && this.#queueTotalSize >= this.#highWaterMark) {
                return;
            }

            this.#pulling = true;
            Promise.resolve(this.#started)
                .then(() => {
                    if (this.#state !== "readable" || this.#closeRequested) {
                        return;
                    }
                    if (typeof this.#source.pull === "function") {
                        return this.#source.pull(this.#controller);
                    }
                })
                .then(
                    () => {
                        this.#pulling = false;
                        if (this.#pullAgain) {
                            this.#pullAgain = false;
                            this.#callPullIfNeeded();
                        }
                    },
                    (error) => {
                        this.#pulling = false;
                        this.#error(error);
                    },
                );
        }
    }

    class WritableStreamDefaultController {
        #error;
        #desiredSize;

        constructor(error, desiredSize) {
            this.#error = error;
            this.#desiredSize = desiredSize;
        }

        get desiredSize() {
            return this.#desiredSize();
        }

        get signal() {
            return undefined;
        }

        error(error) {
            this.#error(error);
        }
    }

    class WritableStreamDefaultWriter {
        #write;
        #closeStream;
        #abort;
        #release;
        #closed;
        #ready;
        #desiredSize;

        constructor(write, closeStream, abort, release, closed, ready, desiredSize) {
            this.#write = write;
            this.#closeStream = closeStream;
            this.#abort = abort;
            this.#release = release;
            this.#closed = closed;
            this.#ready = ready;
            this.#desiredSize = desiredSize;
        }

        get closed() {
            return this.#closed;
        }

        get ready() {
            return this.#ready;
        }

        get desiredSize() {
            return this.#desiredSize();
        }

        write(chunk) {
            return this.#write(chunk);
        }

        close() {
            return this.#closeStream();
        }

        abort(reason) {
            return this.#abort(reason);
        }

        releaseLock() {
            this.#release();
        }
    }

    class WritableStream {
        #sink;
        #strategy;
        #highWaterMark;
        #state = "writable";
        #storedError;
        #writer = null;
        #writerClosed = null;
        #queue = [];
        #writing = false;
        #controller;
        #started;

        constructor(sink = {}, strategy = {}) {
            this.#sink = sink || {};
            this.#strategy = strategy || {};
            this.#highWaterMark = highWaterMark(this.#strategy, 1);
            this.#controller = new WritableStreamDefaultController(
                (error) => this.#error(error),
                () => this.#highWaterMark - this.#queue.length,
            );

            let startResult;
            try {
                startResult =
                    typeof this.#sink.start === "function"
                        ? this.#sink.start(this.#controller)
                        : undefined;
            } catch (error) {
                startResult = Promise.reject(error);
            }

            this.#started = Promise.resolve(startResult).catch((error) => {
                this.#error(error);
            });
        }

        get locked() {
            return this.#writer !== null;
        }

        getWriter() {
            if (this.#writer) {
                throw makeError("TypeError", "stream is already locked");
            }

            this.#writerClosed = makePromise();
            const writer = new WritableStreamDefaultWriter(
                (chunk) => this.#write(chunk),
                () => this.#close(),
                (reason) => this.#abort(reason),
                () => this.#releaseWriter(),
                this.#writerClosed.promise,
                Promise.resolve(),
                () => this.#highWaterMark - this.#queue.length,
            );
            this.#writer = writer;
            return writer;
        }

        abort(reason) {
            return this.#abort(reason);
        }

        #write(chunk) {
            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }
            if (this.#state !== "writable") {
                return Promise.reject(makeError("TypeError", "stream is not writable"));
            }

            return new Promise((resolve, reject) => {
                this.#queue.push({ type: "write", chunk, resolve, reject });
                this.#processQueue();
            });
        }

        #close() {
            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }
            if (this.#state !== "writable") {
                return Promise.reject(makeError("TypeError", "stream is not writable"));
            }

            this.#state = "closing";
            return new Promise((resolve, reject) => {
                this.#queue.push({ type: "close", resolve, reject });
                this.#processQueue();
            });
        }

        #abort(reason) {
            if (this.#state === "closed") {
                return Promise.resolve();
            }
            if (this.#state === "errored") {
                return Promise.reject(this.#storedError);
            }

            this.#state = "closed";
            const error = abortError(reason);
            for (const request of this.#queue) {
                request.reject(error);
            }
            this.#queue = [];

            if (this.#writerClosed) {
                this.#writerClosed.resolve();
            }

            return Promise.resolve(
                typeof this.#sink.abort === "function"
                    ? this.#sink.abort(reason)
                    : undefined,
            ).then(() => undefined);
        }

        #error(error) {
            if (this.#state === "closed" || this.#state === "errored") {
                return;
            }

            this.#state = "errored";
            this.#storedError = error;
            for (const request of this.#queue) {
                request.reject(error);
            }
            this.#queue = [];

            if (this.#writerClosed) {
                this.#writerClosed.reject(error);
            }
        }

        #releaseWriter() {
            this.#writer = null;
            this.#writerClosed = null;
        }

        #processQueue() {
            if (this.#writing || this.#queue.length === 0) {
                return;
            }

            const request = this.#queue.shift();

            if (request.type === "write") {
                if (this.#state !== "writable" && this.#state !== "closing") {
                    request.reject(
                        this.#storedError ||
                            makeError("TypeError", "stream is not writable"),
                    );
                    this.#processQueue();
                    return;
                }

                this.#writing = true;
                Promise.resolve(this.#started)
                    .then(() => {
                        if (this.#state === "errored") {
                            throw this.#storedError;
                        }
                        if (typeof this.#sink.write === "function") {
                            return this.#sink.write(request.chunk, this.#controller);
                        }
                    })
                    .then(
                        () => {
                            this.#writing = false;
                            request.resolve();
                            this.#processQueue();
                        },
                        (error) => {
                            this.#writing = false;
                            request.reject(error);
                            this.#error(error);
                        },
                    );
                return;
            }

            this.#writing = true;
            Promise.resolve(this.#started)
                .then(() => {
                    if (typeof this.#sink.close === "function") {
                        return this.#sink.close();
                    }
                })
                .then(
                    () => {
                        this.#writing = false;
                        this.#state = "closed";
                        request.resolve();
                        if (this.#writerClosed) {
                            this.#writerClosed.resolve();
                        }
                        this.#processQueue();
                    },
                    (error) => {
                        this.#writing = false;
                        request.reject(error);
                        this.#error(error);
                    },
                );
        }
    }

    class TransformStreamDefaultController {
        #readableController;
        #terminated = false;

        constructor(readableController) {
            this.#readableController = readableController;
        }

        get desiredSize() {
            return this.#readableController.desiredSize;
        }

        enqueue(chunk) {
            if (this.#terminated) {
                throw makeError("TypeError", "transform stream is terminated");
            }
            this.#readableController.enqueue(chunk);
        }

        error(reason) {
            if (this.#terminated) {
                return;
            }
            this.#terminated = true;
            this.#readableController.error(reason);
        }

        terminate() {
            if (this.#terminated) {
                return;
            }
            this.#terminated = true;
            this.#readableController.close();
        }
    }

    class TransformStream {
        #transformer;
        #readable;
        #writable;
        #controller;

        constructor(transformer = {}, writableStrategy = {}, readableStrategy = {}) {
            this.#transformer = transformer || {};

            const readable = new ReadableStream(
                {
                    start: (controller) => {
                        this.#controller = new TransformStreamDefaultController(controller);
                        if (typeof this.#transformer.start === "function") {
                            return this.#transformer.start(this.#controller);
                        }
                    },
                    cancel: (reason) => {
                        if (typeof this.#transformer.cancel === "function") {
                            return this.#transformer.cancel(reason);
                        }
                    },
                },
                readableStrategy,
            );

            const writable = new WritableStream(
                {
                    write: (chunk) => {
                        if (typeof this.#transformer.transform === "function") {
                            return this.#transformer.transform(chunk, this.#controller);
                        }
                        this.#controller.enqueue(chunk);
                    },
                    close: () => {
                        const result =
                            typeof this.#transformer.flush === "function"
                                ? this.#transformer.flush(this.#controller)
                                : undefined;
                        return Promise.resolve(result).then(() => {
                            this.#controller.terminate();
                        });
                    },
                    abort: (reason) => {
                        if (typeof this.#transformer.cancel === "function") {
                            return this.#transformer.cancel(reason);
                        }
                    },
                },
                writableStrategy,
            );

            this.#readable = readable;
            this.#writable = writable;
        }

        get readable() {
            return this.#readable;
        }

        get writable() {
            return this.#writable;
        }
    }

    class CountQueuingStrategy {
        constructor(options) {
            this.highWaterMark = options.highWaterMark;
        }

        size() {
            return 1;
        }
    }

    class ByteLengthQueuingStrategy {
        constructor(options) {
            this.highWaterMark = options.highWaterMark;
        }

        size(chunk) {
            return chunk.byteLength === undefined ? 1 : chunk.byteLength;
        }
    }

    Object.assign(globalThis, {
        ReadableStream,
        ReadableByteStreamController,
        ReadableStreamBYOBReader,
        ReadableStreamBYOBRequest,
        ReadableStreamDefaultController,
        ReadableStreamDefaultReader,
        WritableStream,
        WritableStreamDefaultController,
        WritableStreamDefaultWriter,
        TransformStream,
        TransformStreamDefaultController,
        CountQueuingStrategy,
        ByteLengthQueuingStrategy,
    });
})();
