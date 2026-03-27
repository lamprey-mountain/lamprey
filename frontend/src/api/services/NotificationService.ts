import { Message } from "sdk";
import { RootStore } from "../core/Store";
import { notificationPermission } from "../../notification";
import { stripMarkdownAndResolveMentions as stripMarkdownAndResolveMentionsOriginal } from "../../notification-util";
import { generateNotificationIcon } from "../../drawing";

export class NotificationService {
	constructor(private store: RootStore) {}

	async handleMessageCreate(m: Message) {
		const me = this.store.users.get("@self");
		let is_mentioned = false;
		const mentions = (m.latest_version as any).mentions;

		// Determine if mentioned
		if (
			me && m.author_id !== me.id &&
			(m.latest_version as any).type === "DefaultMarkdown" && mentions
		) {
			if (mentions.users?.some((u: any) => u.id === me.id)) {
				is_mentioned = true;
			}
			if (!is_mentioned && mentions.everyone) {
				is_mentioned = true;
			}
			if (!is_mentioned && mentions.roles && mentions.roles.length > 0) {
				const channel = this.store.channels.get(m.channel_id);
				if (channel?.room_id) {
					const room_member = this.store.roomMembers.get(
						`${channel.room_id}:${me.id}`,
					);
					if (room_member && mentions.roles) {
						for (const role of mentions.roles) {
							if (room_member.roles.some((r) => r === role.id)) {
								is_mentioned = true;
								break;
							}
						}
					}
				}
			}
		}

		// We need access to preferences. For now, assume we can get them from store or global
		// Since we haven't fully refactored preferences into RootStore, we might need a bridge
		// or just read from the legacy location if available?
		// Or better, let's inject a preferences accessor into RootStore.

		const preferences = this.store.preferences.read();
		if (!preferences) return;

		if (
			is_mentioned &&
			notificationPermission() === "granted" &&
			preferences.frontend["desktop_notifs"] === "yes"
		) {
			const author = this.store.users.get(m.author_id);
			const channel = this.store.channels.get(m.channel_id);
			const title = `${author?.name ?? "Someone"} in #${
				channel?.name ?? "channel"
			}`;
			const rawContent = (m.latest_version as any).type === "DefaultMarkdown"
				? (m.latest_version as any).content ?? ""
				: "";

			// Helper wrapper for the util
			const processedContent = await stripMarkdownAndResolveMentionsOriginal(
				rawContent,
				m.channel_id,
				this.store as any, // HACK: The util expects 'Api' but RootStore is close enough or we fix util
				(m.latest_version as any).mentions,
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
						URL.revokeObjectURL(icon!);
					};
				}
			})();
		}

		// TTS notifications
		const ttsEnabled = preferences.frontend["tts_notifs"] === "yes";
		const ttsMode = preferences.notifs.tts;
		const shouldSpeak = ttsEnabled && ttsMode !== "Nothing" &&
			(ttsMode === "Always" || (ttsMode === "Mentions" && is_mentioned));
		const isOwnMessage = m.author_id === me?.id;

		if (
			shouldSpeak && !isOwnMessage &&
			(m.latest_version as any).type === "DefaultMarkdown"
		) {
			const author = this.store.users.get(m.author_id);
			const channel = this.store.channels.get(m.channel_id);
			const rawContent = (m.latest_version as any).content ?? "";
			const processedContent = await stripMarkdownAndResolveMentionsOriginal(
				rawContent,
				m.channel_id,
				this.store as any,
				(m.latest_version as any).mentions,
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
