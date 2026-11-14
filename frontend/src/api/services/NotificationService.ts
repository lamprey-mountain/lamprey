import type { Message, MessageVersion } from "sdk";
import { isMarkdown } from "@/components/features/chat/Message";
import { generateNotificationIcon } from "@/lib/drawing";
import { notificationPermission } from "@/lib/notifications";
import {
	globalNotifs,
	isMessageMention,
	isMessageNotified,
	resolveNotifs,
} from "@/lib/notifications/preferences";
import { stripMarkdownAndResolveMentions as stripMarkdownAndResolveMentionsOriginal } from "@/lib/notifications/util";
import type { RootStore } from "../core/Store";

function getContentFromVersion(latestVersion: MessageVersion): string {
	if (isMarkdown(latestVersion.type)) {
		return latestVersion.content ?? "";
	}
	return "";
}

export class NotificationService {
	constructor(private store: RootStore) {}

	async handleMessageCreate(m: Message) {
		const me = this.store.users.get("@self");
		const latestVersion = m.latest_version;
		const mentions = latestVersion?.mentions;
		const isMarkdownMsg = latestVersion
			? isMarkdown(latestVersion.type)
			: false;
		const content = latestVersion ? getContentFromVersion(latestVersion) : "";

		// the channel may not be cached yet (eg. the first message in a new dm)
		const channel = this.store.channels.get(m.channel_id);
		const notifs = channel
			? resolveNotifs(this.store, channel)
			: globalNotifs(this.store);
		const is_dm = channel
			? channel.type === "Dm" || channel.type === "Gdm"
			: !m.room_id;
		const is_mentioned =
			m.author_id !== me?.id &&
			(is_dm ||
				(!!channel && isMessageMention(this.store, channel, m, notifs)));
		const notified = isMessageNotified(notifs, is_dm, is_mentioned);

		const preferences = this.store.preferences.read();
		if (!preferences) return;

		if (
			notified &&
			notificationPermission() === "granted" &&
			preferences.frontend.desktop_notifs
		) {
			const author = this.store.users.get(m.author_id);
			const title = `${author?.name ?? "Someone"} in #${
				channel?.name ?? "channel"
			}`;
			const rawContent = content;

			// Helper wrapper for the util
			const processedContent = await stripMarkdownAndResolveMentionsOriginal(
				rawContent,
				m.channel_id,
				this.store,
				mentions,
			);
			const body = processedContent.substring(0, 200);

			(async () => {
				let icon: string | undefined;
				if (author) {
					const room = channel?.room_id
						? this.store.rooms.get(channel.room_id)
						: undefined;
					const iconBlob = await generateNotificationIcon(
						author,
						room ?? undefined,
					);
					if (iconBlob) {
						icon = URL.createObjectURL(iconBlob);
					}
				}

				const notification = new Notification(title, { body, icon });
				notification.onclick = () => {
					window.focus();
					location.href = `/channel/${m.channel_id}/message/${m.id}`;
				};
				if (icon) {
					notification.onclose = () => {
						URL.revokeObjectURL(icon);
					};
				}
			})();
		}

		// TTS notifications
		const ttsEnabled = preferences.frontend.tts_notifs;
		const ttsMode = preferences.notifs.tts;
		const shouldSpeak =
			notified &&
			ttsEnabled &&
			ttsMode !== "Nothing" &&
			(ttsMode === "Always" || (ttsMode === "Mentions" && is_mentioned));
		const isOwnMessage = m.author_id === me?.id;

		if (shouldSpeak && !isOwnMessage && isMarkdownMsg) {
			const author = this.store.users.get(m.author_id);
			const rawContent = content;
			const processedContent = await stripMarkdownAndResolveMentionsOriginal(
				rawContent,
				m.channel_id,
				this.store,
				mentions,
			);
			const text = processedContent.substring(0, 200);

			const utterance = new SpeechSynthesisUtterance(
				`${author?.name ?? "Someone"} in #${
					channel?.name ?? "channel"
				} says: ${text}`,
			);
			window.speechSynthesis.speak(utterance);
		}
	}
}
