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

  interface AbortSignalEvent {
    readonly type: "abort";
    readonly target: AbortSignal;
    readonly currentTarget: AbortSignal;
    readonly bubbles: false;
    readonly cancelable: false;
    readonly defaultPrevented: boolean;
    readonly timeStamp: number;
  }

  type AbortSignalEventListener = (this: AbortSignal, event: AbortSignalEvent) => void;
  type AbortSignalEventListenerObject = {
    handleEvent(event: AbortSignalEvent): void;
  };

  class AbortSignal {
    static abort(reason?: unknown): AbortSignal;
    static timeout(milliseconds: number): AbortSignal;
    static any(signals: Iterable<AbortSignal>): AbortSignal;

    readonly aborted: boolean;
    readonly reason: unknown;
    onabort: AbortSignalEventListener | null;

    addEventListener(
      type: "abort",
      callback: AbortSignalEventListener | AbortSignalEventListenerObject,
      options?: boolean | { once?: boolean },
    ): void;
    removeEventListener(
      type: "abort",
      callback: AbortSignalEventListener | AbortSignalEventListenerObject,
    ): void;
    dispatchEvent(event: AbortSignalEvent): boolean;
    throwIfAborted(): void;
  }

  class AbortController {
    constructor();
    readonly signal: AbortSignal;
    abort(reason?: unknown): void;
  }

  type AlgorithmIdentifier = string | { name: string };
  type KeyFormat = "raw" | "spki" | "pkcs8" | "jwk";
  type KeyType = "secret" | "private" | "public";
  type KeyUsage =
    | "encrypt"
    | "decrypt"
    | "sign"
    | "verify"
    | "deriveKey"
    | "deriveBits"
    | "wrapKey"
    | "unwrapKey";

  interface JsonWebKey {
    [key: string]: unknown;
  }

  interface CryptoKeyPair {
    privateKey: CryptoKey;
    publicKey: CryptoKey;
  }

  class CryptoKey {
    private constructor();
    readonly algorithm: { name: string; [key: string]: unknown };
    readonly extractable: boolean;
    readonly type: KeyType;
    readonly usages: KeyUsage[];
  }

  class SubtleCrypto {
    private constructor();
    encrypt(
      algorithm: AlgorithmIdentifier,
      key: CryptoKey,
      data: ArrayBuffer | ArrayBufferView,
    ): Promise<ArrayBuffer>;
    decrypt(
      algorithm: AlgorithmIdentifier,
      key: CryptoKey,
      data: ArrayBuffer | ArrayBufferView,
    ): Promise<ArrayBuffer>;
    sign(
      algorithm: AlgorithmIdentifier,
      key: CryptoKey,
      data: ArrayBuffer | ArrayBufferView,
    ): Promise<ArrayBuffer>;
    verify(
      algorithm: AlgorithmIdentifier,
      key: CryptoKey,
      signature: ArrayBuffer | ArrayBufferView,
      data: ArrayBuffer | ArrayBufferView,
    ): Promise<boolean>;
    digest(
      algorithm: AlgorithmIdentifier,
      data: ArrayBuffer | ArrayBufferView,
    ): Promise<ArrayBuffer>;
    generateKey(
      algorithm: AlgorithmIdentifier,
      extractable: boolean,
      keyUsages: KeyUsage[],
    ): Promise<CryptoKey | CryptoKeyPair>;
    deriveKey(
      algorithm: AlgorithmIdentifier,
      baseKey: CryptoKey,
      derivedKeyType: AlgorithmIdentifier,
      extractable: boolean,
      keyUsages: KeyUsage[],
    ): Promise<CryptoKey>;
    deriveBits(
      algorithm: AlgorithmIdentifier,
      baseKey: CryptoKey,
      length: number,
    ): Promise<ArrayBuffer>;
    importKey(
      format: KeyFormat,
      keyData: ArrayBuffer | ArrayBufferView | JsonWebKey,
      algorithm: AlgorithmIdentifier,
      extractable: boolean,
      keyUsages: KeyUsage[],
    ): Promise<CryptoKey>;
    exportKey(
      format: KeyFormat,
      key: CryptoKey,
    ): Promise<ArrayBuffer | JsonWebKey>;
    wrapKey(
      format: KeyFormat,
      key: CryptoKey,
      wrappingKey: CryptoKey,
      wrapAlgorithm: AlgorithmIdentifier,
    ): Promise<ArrayBuffer>;
    unwrapKey(
      format: KeyFormat,
      wrappedKey: ArrayBuffer | ArrayBufferView,
      unwrappingKey: CryptoKey,
      unwrapAlgorithm: AlgorithmIdentifier,
      unwrappedKeyAlgorithm: AlgorithmIdentifier,
      extractable: boolean,
      keyUsages: KeyUsage[],
    ): Promise<CryptoKey>;
  }

  class Crypto {
    private constructor();
    readonly subtle: SubtleCrypto;
    getRandomValues<T extends ArrayBufferView>(array: T): T;
    randomUUID(): string;
  }

  const crypto: Crypto;

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

  interface TextEncoderEncodeIntoResult {
    read: number;
    written: number;
  }

  interface TextDecoderOptions {
    fatal?: boolean;
    ignoreBOM?: boolean;
  }

  interface TextDecoderDecodeOptions {
    stream?: boolean;
  }

  class TextEncoder {
    constructor();
    readonly encoding: "utf-8";
    encode(input?: string): Uint8Array;
    encodeInto(source: string, destination: Uint8Array): TextEncoderEncodeIntoResult;
  }

  class TextDecoder {
    constructor(label?: string, options?: TextDecoderOptions);
    readonly encoding: string;
    readonly fatal: boolean;
    readonly ignoreBOM: boolean;
    decode(
      input?: ArrayBuffer | ArrayBufferView,
      options?: TextDecoderDecodeOptions,
    ): string;
  }

  class TextEncoderStream extends TransformStream<string, Uint8Array> {
    constructor();
    readonly encoding: "utf-8";
  }

  class TextDecoderStream extends TransformStream<ArrayBufferView, string> {
    constructor(label?: string, options?: TextDecoderOptions);
    readonly encoding: string;
    readonly fatal: boolean;
    readonly ignoreBOM: boolean;
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

  interface UnderlyingByteSource {
    type: "bytes";
    autoAllocateChunkSize?: number;
    start?(controller: ReadableByteStreamController): void | PromiseLike<void>;
    pull?(controller: ReadableByteStreamController): void | PromiseLike<void>;
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

  class ReadableStreamBYOBRequest {
    readonly view: Uint8Array;
    respond(bytesWritten: number): void;
    respondWithNewView(view: ArrayBufferView): void;
  }

  class ReadableByteStreamController {
    readonly byobRequest: ReadableStreamBYOBRequest | null;
    readonly desiredSize: number | null;
    enqueue(chunk: ArrayBufferView): void;
    close(): void;
    error(reason?: unknown): void;
  }

  class ReadableStreamBYOBReader {
    readonly closed: Promise<void>;
    read<T extends ArrayBufferView>(view: T): Promise<ReadableStreamReadResult<T>>;
    cancel(reason?: unknown): Promise<void>;
    releaseLock(): void;
  }

  class ReadableStream<R = unknown> implements AsyncIterable<R> {
    constructor(underlyingSource: UnderlyingByteSource, strategy?: QueuingStrategy<ArrayBufferView>);
    constructor(
      underlyingSource?: UnderlyingSource<R>,
      strategy?: QueuingStrategy<R>,
    );

    readonly locked: boolean;
    cancel(reason?: unknown): Promise<void>;
    getReader(options?: { mode?: undefined }): ReadableStreamDefaultReader<R>;
    getReader(options: { mode: "byob" }): ReadableStreamBYOBReader;
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
