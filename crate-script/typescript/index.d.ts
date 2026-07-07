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
}

/** http types */
declare module "lamprey:http" {
  // ...
}
