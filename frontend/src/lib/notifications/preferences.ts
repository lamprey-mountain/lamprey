import type { Channel, Message, Preferences } from "sdk";
import type { RootStore } from "@/api/core/Store";
import { getDate } from "@/utils/general";

type NotifsGlobal = Preferences["notifs"];
type NotifsMessages = NotifsGlobal["messages"];
type NotifsReplies = NotifsGlobal["replies"];
type NotifsThreads = NotifsGlobal["threads"];

export type ResolvedNotifs = {
	muted: boolean;
	messages: NotifsMessages;
	replies: NotifsReplies;
	threads: NotifsThreads;
	mention_everyone: boolean;
	mention_roles: boolean;
};

/** whether a mute is currently in effect */
export function isMuteActive(mute: NotifsGlobal["mute"] | undefined): boolean {
	if (!mute) return false;
	if (!mute.expires_at) return true;
	return getDate(mute.expires_at).getTime() > Date.now();
}

/** notification preferences when the channel isn't cached yet (eg. a new dm) */
export function globalNotifs(store: RootStore): ResolvedNotifs {
	const global = store.preferences.read().notifs;

	return {
		muted: isMuteActive(global.mute),
		messages: global.messages,
		replies: global.replies,
		threads: global.threads,
		mention_everyone: true,
		mention_roles: true,
	};
}

/** resolve notification preferences for a channel (channel -> room -> global) */
export function resolveNotifs(
	store: RootStore,
	channel: Channel,
): ResolvedNotifs {
	const global = store.preferences.read().notifs;
	const room = channel.room_id
		? store.rooms.get(channel.room_id)?.preferences?.notifs
		: undefined;
	const ch = channel.preferences?.notifs;

	return {
		muted:
			isMuteActive(ch?.mute) ||
			isMuteActive(room?.mute) ||
			isMuteActive(global.mute),
		messages: ch?.messages ?? room?.messages ?? global.messages,
		replies: ch?.replies ?? room?.replies ?? global.replies,
		threads: ch?.threads ?? room?.threads ?? global.threads,
		mention_everyone: room?.mention_everyone ?? true,
		mention_roles: room?.mention_roles ?? true,
	};
}

/**
 * whether a message should send a notification, following the channel's
 * `messages` setting. direct messages always notify unless muted.
 */
export function isMessageNotified(
	notifs: ResolvedNotifs,
	is_dm: boolean,
	is_mentioned: boolean,
): boolean {
	if (notifs.muted) return false;
	if (is_dm) return true;

	switch (notifs.messages) {
		case "Everything":
			return true;
		case "Watching":
		case "Mentions":
			return is_mentioned;
		case "Nothing":
			return false;
	}
}

/** whether a message mentions the current user, respecting room mention settings */
export function isMessageMention(
	store: RootStore,
	channel: Channel,
	message: Message,
	notifs: ResolvedNotifs,
): boolean {
	const session = store.session();
	const user_id =
		session && session.status !== "Unauthorized" ? session.user_id : undefined;
	if (!user_id) return false;

	// direct messages always notify
	if (channel.type === "Dm" || channel.type === "Gdm") return true;

	const mentions = message.latest_version.mentions;
	if (!mentions) return false;

	if (mentions.users?.some((u) => u.id === user_id)) return true;
	if (mentions.everyone && notifs.mention_everyone) return true;

	if (mentions.roles?.length && notifs.mention_roles && channel.room_id) {
		const member = store.roomMembers.get(`${channel.room_id}:${user_id}`);
		if (
			member?.roles.some((role_id) =>
				mentions.roles?.some((role) => role.id === role_id),
			)
		) {
			return true;
		}
	}

	return false;
}
