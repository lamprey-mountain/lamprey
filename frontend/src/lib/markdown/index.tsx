import init, { Parsed, Parser } from "@lamprey/markdown";
import { createResource } from "solid-js";

export const loaded = init();
export const [loadedResource] = createResource(() => loaded);
export { countEmojiOnly } from "./emoji";

export const parser = loaded.then(() => new Parser());
export const [parserResource] = createResource(() => parser);

export { Parsed, Parser };
