export * from "./client.ts";
export * from "./messages.ts";
export type { paths } from "./schema.d.ts";
export * as types from "./types.ts"; // TODO: deprecate/remove?
export * from "./types.ts"; // TODO: deprecate/remove?
export * from "./upload.ts";

// raw api types
import type { components } from "./schema.d.ts";
export type api = components["schemas"];

// api v2
export { Lamprey } from "./client/client.ts";
// core types used everywhere in the sdk
export { Emitter } from "./core/events.ts";
export {
	bytesToUuid,
	getTimestampFromUUID,
	isUuid,
	SERVER_ROOM_ID,
	uuidToBytes,
} from "./core/uuid.ts";
import "./client/shared-worker/client.ts";
