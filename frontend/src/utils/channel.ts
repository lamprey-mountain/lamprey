import type { Channel, ChannelType, Preferences } from "sdk";
import type { SetStoreFunction } from "solid-js/store";
import type { ChannelState } from "../contexts/channel";

/**
 * Get the channel type category for thread sidebar preferences
 */
export function getChannelTypeCategory(
	channelType: ChannelType,
): "text" | "document" | "forum" | null {
	if (
		channelType === "Text" ||
		channelType === "Announcement" ||
		channelType === "Dm" ||
		channelType === "Gdm"
	) {
		return "text";
	}
	if (channelType === "Document" || channelType === "Wiki") {
		return "document";
	}
	if (channelType === "Forum" || channelType === "Forum2") {
		return "forum";
	}
	return null;
}

/**
 * Check if threads should open in sidebar for a given channel based on user preferences
 */
export function shouldUseThreadSidebar(
	channel: Channel,
	preferences: Preferences,
): boolean {
	const category = getChannelTypeCategory(channel.type);
	if (!category) return false;

	const prefKey = `threads_sidebar_${category}`;
	return preferences.frontend[prefKey] === "yes";
}

/**
 * Open a thread in the sidebar if the preference is enabled, otherwise navigate to the thread page
 */
export function openThread(
	thread: Channel,
	parentChannel: Channel,
	preferences: Preferences,
	setChannelState: SetStoreFunction<ChannelState>,
	navigate: (path: string) => void,
) {
	const shouldUseSidebar = shouldUseThreadSidebar(parentChannel, preferences);
	if (shouldUseSidebar) {
		setChannelState("thread_chat_sidebar_thread_id", thread.id);
	} else {
		navigate(`/thread/${thread.id}`);
	}
}
