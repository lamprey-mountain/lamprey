import type { ChannelType } from "sdk";
import type { Api } from "../api";
import { canUseCommand as checkCommandPermission } from "../permission-calculator";

/**
 * Check if a command should be available based on channel type and permissions
 */
export function canUseCommand(
	api: Api,
	room_id: string | undefined,
	channel: any,
	commandName: string,
): boolean {
	const self_id = api.users.cache.get("@self")?.id;
	if (!self_id) return false;

	// Use the centralized permission calculator
	return checkCommandPermission(
		{ api, room_id, channel_id: channel?.id },
		self_id,
		commandName,
		channel,
	);
}
