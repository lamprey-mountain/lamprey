import { UUID } from "uuidv7";

export type Uuid = string & { readonly __uuid: "Uuid" };

/** serialize a string uuid to raw bytes */
export function uuidToBytes(uuid: Uuid): Uint8Array {
	const hex = uuid.replace(/-/g, "");
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 16; i++) {
		bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
	}
	return bytes;
}

/** serialize bytes into a string uuid */
export function bytesToUuid(bytes: Uint8Array): Uuid {
	const hex = [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
	return [
		hex.substring(0, 8),
		hex.substring(8, 12),
		hex.substring(12, 16),
		hex.substring(16, 20),
		hex.substring(20, 32),
	].join("-") as Uuid;
}

/** check if a string represents a valid uuid */
export function isUuid(uuid: string): uuid is Uuid {
	const regex =
		/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	return regex.test(uuid);
}

/** get the timestamp of a uuid (can be passed to new Date(_)) */
export function getTimestampFromUuid(uuid: Uuid): number {
	const timestamp = UUID.parse(uuid)
		.bytes.slice(0, 6)
		.reduce((acc: number, e: number) => acc * 256 + e, 0);
	return timestamp;
}

export const UUID_MIN = "00000000-0000-0000-0000-000000000000" as Uuid;
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff" as Uuid;

// TODO: export more uuid consts
// see crate-common/src/v1/types/ids.rs
export const SERVER_ROOM_ID = "00000000-0000-7000-0000-736572766572" as Uuid;
