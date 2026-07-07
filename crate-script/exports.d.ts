// for use with the scripting api

// declare const self: Globals;

export type Globals = {
  log: {
    debug(content: string, metadata: Record<string, string>): void;
    info(content: string, metadata: Record<string, string>): void;
    warn(content: string, metadata: Record<string, string>): void;
    error(content: string, metadata: Record<string, string>): void;
  }
};

export type Context = {
  // none of these exist yet
  // fs: {};
  // net: {};
  // env: {};
  request: Request;
};

export type Capability = keyof Context;

type FilteredContext<T extends Capability[]> = Pick<Context, T[number]>;

export type Register = {
  /** basic input, must be manually triggered */
  onTrigger(): Input<[]>;

  /** http input, must be manually triggered */
	onHttp(): Input<["request"], Response | Promise<Response>>;
}

export type Input<T extends Capability[], R = void> = {
  needs<U extends Capability[]>(perms: [...U]): Input<[...T, ...U]>;
  id(id: string): Input<T>;
  label(id: string): Input<T>;
  run(call: (ctx: FilteredContext<T>) => R): void;
};

export interface Script {
  name: string;
  description?: string;
  homepage_url?: string;
  authors?: string[]; // TEMP(?): maybe use an object with user ids, urls, etc
  version?: string;
  license?: string;

  register(ctl: Register): void;
}

/** http types */
declare module "lamprey:http" {
  // NOTE: i should probably make these use actual http methods
  export class Request {
    readonly url: string;
    readonly method: "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | string;
    readonly redirect: string;
    readonly headers: Headers;

    // compat?
    readonly statusText: string;

    // extra?
    readonly path: string;

    // readonly body: ReadableStream | null;
    // json(): Promise<any>;
    // text(): Promise<string>;
    // blob, arraybuffer
  }

  export class FetchInit extends Request {
    // cache?: no-store, no-cache
  }

  export class Response {
    readonly body: unknown;
    readonly headers: unknown;
    readonly status: number;
    readonly url: string;

    // compat?
    readonly statusText: string;
    readonly bodyUsed: boolean;
    readonly ok: boolean;
    readonly redirected: boolean;

    // extra?
    // network info (ip addr)

    // constructor(body?: BodyInit, init?: ResponseInit);
    // static json(data: any, init?: ResponseInit): Response;

    // readonly body: ReadableStream | null;
    // json(): Promise<any>;
    // text(): Promise<string>;
    // blob, arraybuffer
  }

  export class Headers {
    // copy web standards
    // append, delete, get, has, set, entries, keys, values, forEach
    // also getSetCookie?
  }

  // TODO
  export class Router {
    // write server router in rust for performance
  }

  // probably copy https://developers.cloudflare.com/workers/runtime-apis/fetch/
}

/** starting and managing other processes */
declare module "lamprey:redex" {
    export class RedexManager {
      // lookupSelf()
      // lookupRedex()
      // lookupEval()
    }

    /** a piece of runnable code */
    export class Redex {
      // id
      // spawn()
    }

    /** a currently running redex */
    export class Eval {
      // id, redexId
      // stop()
    }
}

declare global {

}

export {};
