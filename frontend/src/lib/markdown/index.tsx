import init, { Parser } from "@lamprey/markdown";
import { createResource } from "solid-js";

export const loaded = init();
export const [loadedResource] = createResource(() => loaded);
export { Parser };

export * from "./old";
export { countEmojiOnly } from "./emoji";
