// for use with the scripting api

export type Context = {
  fs: {};
  net: {};
  env: {};
};

export type Capability = keyof Context;

type FilteredContext<T extends Capability[]> = Pick<Context, T[number]>;

export type Register = {
  /** basic input, must be manually triggered */
  onTrigger(): Input<[]>;
}

export type Input<T extends Capability[]> = {
  needs<U extends Capability[]>(perms: [...U]): Input<[...T, ...U]>;
  id(id: string): Input<T>;
  label(id: string): Input<T>;
  run(call: (ctx: FilteredContext<T>) => void): void;
};

export default {
  name: "my script name",
  register(ctl: Register) {
    ctl
      .onTrigger()
      .id("custom_id")
      .label("Custom Label")
      .needs(["fs", "net"])
      .run(({ fs, net }) => {
        // something here...
      })
  }
}
