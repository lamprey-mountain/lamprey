import { UUID } from "uuidv7";

export * as types from "./types.ts";
export * from "./types.ts";
export * from "./client.ts";
export * from "./upload.ts";

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes.slice(0, 6).reduce(
		(acc: number, e: number) => acc * 256 + e,
		0,
	);
	return new Date(timestamp);
}

export const SERVER_ROOM_ID = "00000000-0000-7000-0000-736572766572";
