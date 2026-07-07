import { UUID } from "uuidv7";

// TODO: use (reexport?) UUID module instead of these functions
// remove uuidToBytes
// remove bytesToUuid
// use getTimestampFromUUID2 instead of getTimestampFromUUID

export function uuidToBytes(uuid: string): Uint8Array {
	const hex = uuid.replace(/-/g, "");
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 16; i++) {
		bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
	}
	return bytes;
}

export function bytesToUuid(bytes: Uint8Array): string {
	const hex = [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
	return [
		hex.substring(0, 8),
		hex.substring(8, 12),
		hex.substring(12, 16),
		hex.substring(16, 20),
		hex.substring(20, 32),
	].join("-");
}

export function isUuid(uuid: string): boolean {
	const regex =
		/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	return regex.test(uuid);
}

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes
		.slice(0, 6)
		.reduce((acc: number, e: number) => acc * 256 + e, 0);
	return new Date(timestamp);
}

export function getTimestampFromUUID2(uuid: UUID): number {
	const timestamp = uuid.bytes
		.slice(0, 6)
		.reduce((acc: number, e: number) => acc * 256 + e, 0);
	return timestamp;
}

export const UUID_MIN = "00000000-0000-0000-0000-000000000000" as Uuid;
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff" as Uuid;

// TODO: export more uuid consts
// see crate-common/src/v1/types/ids.rs
export const SERVER_ROOM_ID = "00000000-0000-7000-0000-736572766572" as Uuid;

// NOTE: how would this interact with UUID?
export type Uuid = string & { readonly __uuid: "Uuid" };
