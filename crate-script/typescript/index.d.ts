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
}

/** http types */
declare module "lamprey:http" {
  // ...
}
