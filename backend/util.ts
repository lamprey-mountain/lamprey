import { UUID } from "uuidv7";

export const UUID_MIN = "00000000-0000-0000-0000-000000000000";
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes.slice(0, 6).reduce(
		(acc: number, e: number) => acc * 256 + e,
		0,
	);
	return new Date(timestamp);
}
