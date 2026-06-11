// for use with the scripting api

declare const self: Globals;

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
