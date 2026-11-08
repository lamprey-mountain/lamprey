/// <reference no-default-lib="true"/>
/// <reference lib="es2024" />

export { };

type LogFn = (content: string, metadata: Record<string, string>) => void;

declare global {
  const log: {
    debug: LogFn;
    info: LogFn;
    warn: LogFn;
    error: LogFn;
  }

  class Blob {
    constructor(
      parts?: (Blob | ArrayBuffer | ArrayBufferView | string)[],
      options?: { type?: string },
    );

    readonly size: number;
    readonly type: string;

    text(): Promise<string>;
    arrayBuffer(): Promise<ArrayBuffer>;
    bytes(): Promise<Uint8Array>;
    slice(start?: number, end?: number, contentType?: string): Blob;
    stream(): ReadableStream<Uint8Array>;
  }

  class File extends Blob {
    constructor(
      fileBits: (Blob | ArrayBuffer | ArrayBufferView | string)[],
      fileName: string,
      options?: { type?: string; lastModified?: number },
    );

    readonly name: string;
    readonly lastModified: number;
  }

  interface QueuingStrategy<T = unknown> {
    highWaterMark?: number;
    size?(chunk: T): number;
  }

  interface StreamPipeOptions {
    preventAbort?: boolean;
    preventCancel?: boolean;
    preventClose?: boolean;
  }

  interface UnderlyingSource<R = unknown> {
    start?(controller: ReadableStreamDefaultController<R>): void | PromiseLike<void>;
    pull?(controller: ReadableStreamDefaultController<R>): void | PromiseLike<void>;
    cancel?(reason?: unknown): void | PromiseLike<void>;
  }

  interface UnderlyingSink<W = unknown> {
    start?(controller: WritableStreamDefaultController): void | PromiseLike<void>;
    write?(chunk: W, controller: WritableStreamDefaultController): void | PromiseLike<void>;
    close?(): void | PromiseLike<void>;
    abort?(reason?: unknown): void | PromiseLike<void>;
  }

  interface Transformer<I = unknown, O = I> {
    start?(controller: TransformStreamDefaultController<O>): void | PromiseLike<void>;
    transform?(
      chunk: I,
      controller: TransformStreamDefaultController<O>,
    ): void | PromiseLike<void>;
    flush?(controller: TransformStreamDefaultController<O>): void | PromiseLike<void>;
    cancel?(reason?: unknown): void | PromiseLike<void>;
  }

  type ReadableStreamReadResult<T> =
    | { done: false; value: T }
    | { done: true; value?: undefined };

  class ReadableStreamDefaultController<R = unknown> {
    readonly desiredSize: number | null;
    enqueue(chunk: R): void;
    close(): void;
    error(reason?: unknown): void;
  }

  class ReadableStreamDefaultReader<R = unknown> {
    readonly closed: Promise<void>;
    read(): Promise<ReadableStreamReadResult<R>>;
    cancel(reason?: unknown): Promise<void>;
    releaseLock(): void;
  }

  class ReadableStream<R = unknown> implements AsyncIterable<R> {
    constructor(
      underlyingSource?: UnderlyingSource<R>,
      strategy?: QueuingStrategy<R>,
    );

    readonly locked: boolean;
    cancel(reason?: unknown): Promise<void>;
    getReader(options?: { mode?: undefined }): ReadableStreamDefaultReader<R>;
    pipeTo(destination: WritableStream<R>, options?: StreamPipeOptions): Promise<void>;
    pipeThrough<T>(
      transform: { readable: ReadableStream<T>; writable: WritableStream<R> },
      options?: StreamPipeOptions,
    ): ReadableStream<T>;
    tee(): [ReadableStream<R>, ReadableStream<R>];
    values(options?: { preventCancel?: boolean }): AsyncIterableIterator<R>;
    [Symbol.asyncIterator](options?: { preventCancel?: boolean }): AsyncIterableIterator<R>;
  }

  class WritableStreamDefaultController {
    readonly desiredSize: number | null;
    readonly signal: undefined;
    error(reason?: unknown): void;
  }

  class WritableStreamDefaultWriter<W = unknown> {
    readonly closed: Promise<void>;
    readonly ready: Promise<void>;
    readonly desiredSize: number | null;
    write(chunk: W): Promise<void>;
    close(): Promise<void>;
    abort(reason?: unknown): Promise<void>;
    releaseLock(): void;
  }

  class WritableStream<W = unknown> {
    constructor(underlyingSink?: UnderlyingSink<W>, strategy?: QueuingStrategy<W>);

    readonly locked: boolean;
    getWriter(): WritableStreamDefaultWriter<W>;
    abort(reason?: unknown): Promise<void>;
  }

  class TransformStreamDefaultController<O = unknown> {
    readonly desiredSize: number | null;
    enqueue(chunk: O): void;
    error(reason?: unknown): void;
    terminate(): void;
  }

  class TransformStream<I = unknown, O = I> {
    constructor(
      transformer?: Transformer<I, O>,
      writableStrategy?: QueuingStrategy<I>,
      readableStrategy?: QueuingStrategy<O>,
    );

    readonly readable: ReadableStream<O>;
    readonly writable: WritableStream<I>;
  }

  class CountQueuingStrategy implements QueuingStrategy {
    constructor(options: { highWaterMark: number });
    readonly highWaterMark: number;
    size(): number;
  }

  class ByteLengthQueuingStrategy implements QueuingStrategy<ArrayBufferView> {
    constructor(options: { highWaterMark: number });
    readonly highWaterMark: number;
    size(chunk: ArrayBufferView): number;
  }
}

/** http types */
declare module "lamprey:http" {
  // ...
}
