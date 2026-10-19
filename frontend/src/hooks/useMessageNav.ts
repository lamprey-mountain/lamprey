import { useApi, useChannels, useMessages } from "@/api";
import { areReactionKeysEqual } from "@/api/services/ReactionsService.ts";
import { useCtx } from "@/app/context";
import { asMarkdown2, isMarkdown } from "@/components/features/chat/util";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { useReadTracking } from "@/contexts/read-tracking";
import { usePermissions } from "@/hooks/usePermissions";

/**
 * Keyboard navigation for messages in the current channel.
 */
export const useMessageNav = (channel_id: () => string) => {
	const api2 = useApi();
	const channels2 = useChannels();
	const messages = useMessages();
	const [ch, chUpdate] = useChannel()!;
	const readTracking = useReadTracking();
	const currentUser = useCurrentUser();
	const [, modalCtl] = useModals();
	const ctx = useCtx();

	const channel = () => channels2.cache.get(channel_id());
	const perms = usePermissions(
		() => currentUser()?.id,
		() => channel()?.room_id ?? undefined,
		() => channel_id(),
	);

	const getMessages = () =>
		ch.timelineState.messages?.items ??
		messages._ranges.get(channel_id())?.live.items ??
		[];

	const focusedMessage = () => {
		const id = ch.focusedMessageId;
		return id ? messages.cache.get(id) : undefined;
	};

	const focusedEl = () => {
		const id = ch.focusedMessageId;
		if (!id) return null;
		const container = document.querySelector(
			`.chat[data-channel-id="${CSS.escape(channel_id())}"]`,
		);
		const el = container?.querySelector(
			`[data-message-id="${CSS.escape(id)}"]`,
		);
		return el instanceof HTMLElement ? el : null;
	};

	// NOTE: maybe i want the last *fully* visible message?
	const lastVisibleMessageId = () => {
		const container = document.querySelector<HTMLElement>(
			`.chat[data-channel-id="${CSS.escape(channel_id())}"]`,
		);
		const scrollEl = container?.querySelector<HTMLElement>(".timeline");
		if (!scrollEl) return undefined;

		const viewport = scrollEl.getBoundingClientRect();
		const els = scrollEl.querySelectorAll<HTMLElement>(
			".timeline-item > [data-message-id]",
		);
		let last: string | undefined;
		for (let i = 0; i < els.length; i++) {
			const rect = els[i].getBoundingClientRect();
			if (rect.bottom > viewport.top && rect.top < viewport.bottom) {
				last = els[i].dataset.messageId ?? last;
			}
		}
		return last;
	};

	const focusInput = () => {
		if (ch.input_focus) {
			ch.input_focus();
			return;
		}

		// in case the composer didn't register its focus handler yet
		document
			.querySelector<HTMLElement>(
				`.chat[data-channel-id="${CSS.escape(channel_id())}"] .message-input .ProseMirror`,
			)
			?.focus();
	};

	const enter = () => {
		chUpdate("messageNavActive", true);
		chUpdate("focusedMessageId", undefined);
	};

	/** leave keyboard nav without moving focus back to the composer */
	const deactivate = () => {
		chUpdate("messageNavActive", false);
		chUpdate("focusedMessageId", undefined);
	};

	/** scroll to the newest message and mark the channel as read */
	const jumpToEnd = () => {
		ch.timelineState.controller.jumpToBottom();

		const c = channel();
		const message_id = c?.last_message_id;
		const read_id = c?.last_read_id;
		if (message_id && message_id !== read_id) {
			readTracking.ack(channel_id(), message_id, true, false);
		}
	};

	const exit = () => {
		deactivate();
		focusInput();
	};

	const navigate = (direction: 1 | -1) => {
		const items = getMessages();
		const currentId = ch.focusedMessageId;

		if (!currentId) {
			if (direction === -1) {
				const id = lastVisibleMessageId() ?? items[items.length - 1]?.id;
				if (id) chUpdate("focusedMessageId", id);
			}
			return;
		}

		const index = items.findIndex((m) => m.id === currentId);
		if (index === -1) {
			if (direction === -1) {
				const id = lastVisibleMessageId() ?? items[items.length - 1]?.id;
				if (id) chUpdate("focusedMessageId", id);
			} else {
				chUpdate("focusedMessageId", undefined);
				focusInput();
			}
			return;
		}

		const next = index + direction;
		if (next < 0) return;
		if (next >= items.length) {
			chUpdate("focusedMessageId", undefined);
			focusInput();
			return;
		}

		chUpdate("focusedMessageId", items[next].id);
	};

	/** mark the focused message (and everything after it) unread */
	const markUnread = () => {
		const id = ch.focusedMessageId;
		if (!id) return;

		const items = getMessages();
		const index = items.findIndex((m) => m.id === id);
		if (index === -1) return;

		const previous = items[index - 1];
		readTracking.ack(channel_id(), previous?.id ?? id, true, false);
	};

	const reply = () => {
		const id = ch.focusedMessageId;
		if (!id) return;

		const message = messages.cache.get(id);
		if (!message || !isMarkdown(message.latest_version.type)) return;

		chUpdate("reply_id", id);
		chUpdate("messageNavActive", false);
		chUpdate("focusedMessageId", undefined);
		focusInput();
	};

	const canEdit = () => {
		const message = focusedMessage();
		return (
			!!message &&
			isMarkdown(message.latest_version.type) &&
			!message.is_local &&
			message.author_id === currentUser()?.id
		);
	};

	const canPin = () => perms.has("MessagePin");

	const canDelete = () => {
		const message = focusedMessage();
		return (
			!!message &&
			(perms.has("MessageDelete") || message.author_id === currentUser()?.id)
		);
	};

	const canAddReaction = () => perms.has("ReactionAdd");

	const edit = () => {
		const id = ch.focusedMessageId;
		if (!id || !canEdit()) return;

		chUpdate("messageNavActive", false);
		chUpdate("focusedMessageId", undefined);
		chUpdate("editingMessage", {
			message_id: id,
			selection: "end",
		});
	};

	const deleteMessage = () => {
		const message = focusedMessage();
		if (!message || !canDelete()) return;

		modalCtl.confirm("really delete?", async (confirmed) => {
			if (!confirmed) return;
			try {
				await api2.client.http.DELETE(
					"/api/v1/channel/{channel_id}/message/{message_id}",
					{
						params: {
							path: {
								channel_id: channel_id(),
								message_id: message.id,
							},
						},
					},
				);
			} catch (e) {
				console.error("failed to delete message", e);
				return;
			}
			deactivate();
			focusInput();
		});
	};

	const togglePin = () => {
		const message = focusedMessage();
		if (!message || !canPin()) return;

		if (message.pinned) {
			messages.unpin(channel_id(), message.id);
		} else {
			messages.pin(channel_id(), message.id);
		}
	};

	const copyContent = () => {
		const message = focusedMessage();
		if (!message) return false;

		// if the user explicitly selected text, don't intercept it
		const selection = window.getSelection();
		if (
			selection &&
			!selection.isCollapsed &&
			selection.toString().length > 0
		) {
			return false;
		}

		const content = asMarkdown2(message.latest_version)?.content;
		if (content) navigator.clipboard.writeText(content);
		return true;
	};

	const openReactionPicker = () => {
		const message = focusedMessage();
		if (!message || !canAddReaction()) return;

		// open at the toolbar's reaction button (the toolbar is shown while the
		// message is keyboard focused)
		const button = document.querySelector(
			`[data-message-toolbar="${CSS.escape(message.id)}"] button[title="Add reaction"]`,
		) as HTMLElement | null;
		if (button) {
			button.click();
			return;
		}

		const el = focusedEl();
		if (!el) return;

		ctx.setPopout({
			id: "emoji",
			ref: el,
			placement: "left-start",
			props: {
				selected: (emoji: string | null, keepOpen: boolean) => {
					if (emoji) {
						const existing = message.reactions?.find((r) =>
							areReactionKeysEqual(r.key, { type: "Text", content: emoji }),
						);
						if (!existing || !existing.self) {
							api2.reactions.add(channel_id(), message.id, `t:${emoji}`);
						}
					}
					if (!keepOpen) ctx.setPopout(null);
				},
			},
		});
	};

	return {
		enter,
		exit,
		deactivate,
		jumpToEnd,
		navigate,
		markUnread,
		reply,
		edit,
		deleteMessage,
		togglePin,
		copyContent,
		openReactionPicker,
	};
};
