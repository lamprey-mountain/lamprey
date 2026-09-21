import { useLocation, useNavigate } from "@solidjs/router";
import type { EditorView } from "prosemirror-view";
import type { Media, Tag } from "sdk";
import {
	createContext,
	createEffect,
	on,
	type ParentProps,
	untrack,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi, useChannels, useRooms } from "@/api";

export type ChannelTypeOption =
	| "Text"
	| "Voice"
	| "Category"
	| "Forum"
	| "Calendar"
	| "Document"
	| "Wiki";

export type Modal =
	| {
			type: "room_settings";
			room_id: string;
			page?: string;
	  }
	| {
			type: "channel_settings";
			channel_id: string;
			page?: string;
	  }
	| {
			type: "user_settings";
			page?: string;
	  }
	| {
			type: "alert";
			text: string;
	  }
	| {
			type: "confirm";
			text: string;
			cont: (confirmed: boolean) => void;
	  }
	| {
			type: "prompt";
			text: string;
			cont: (text: string | null) => void;
	  }
	| {
			type: "media";
			media: Media;
	  }
	| {
			type: "message_edits";
			channel_id: string;
			message_id: string;
	  }
	| {
			type: "reset_password";
	  }
	| {
			type: "palette";
	  }
	| {
			type: "channel_create";
			room_id: string;
	  }
	| {
			type: "tag_editor";
			forumChannelId: string;
			tag?: Tag;
			onSave?: (tag: Tag) => void;
			onClose?: () => void;
	  }
	| {
			type: "export_data";
	  }
	| {
			type: "view_reactions";
			channel_id: string;
			message_id: string;
	  }
	| {
			type: "privacy";
			room_id: string;
	  }
	| {
			type: "notifications";
			room_id: string;
	  }
	| {
			type: "invite_create";
			room_id?: string;
			channel_id?: string;
	  }
	| {
			type: "attachment";
			channel_id: string;
			local_id: string;
	  }
	| {
			type: "channel_topic";
			channel_id: string;
	  }
	| {
			type: "link";
			editor: EditorView;
	  }
	| {
			type: "kick";
			room_id: string;
			user_id: string;
	  }
	| {
			type: "ban";
			room_id: string;
			user_id?: string;
	  }
	| {
			type: "timeout";
			room_id: string;
			user_id: string;
	  }
	| {
			type: "camera_preview";
			stream: MediaStream;
	  }
	| {
			type: "room_create_or_join";
	  }
	| {
			type: "emoji_upload";
			room_id: string;
			media: Media;
	  }
	| {
			type: "thread_create";
			room_id: string;
			channel_id: string;
	  };

export type ModalsController = {
	close: () => void;
	open: (modal: Modal) => void;
	replace: (modal: Modal) => void;
	alert: (text: string) => void;
	prompt: (text: string, cont: (text: string | null) => void) => void;
	confirm: (text: string, cont: (confirmed: boolean) => void) => void;
};

type ModalsContextType = [Modal[], ModalsController];

const ModalsContext = createContext<ModalsContextType>();

export const ModalsProvider = (p: ParentProps) => {
	const [modals, setModals] = createStore<Modal[]>([]);

	const controller: ModalsController = {
		close() {
			setModals((prev) => prev.slice(1));
		},
		open(modal: Modal) {
			setModals((prev) => [...prev, modal]);
		},
		replace(modal: Modal) {
			if (modals.at(-1)?.type === modal.type) {
				setModals(modals.length - 1, modal);
			} else {
				setModals((prev) => [...prev.slice(0, -1), modal]);
			}
		},
		alert(text: string) {
			setModals((prev) => [{ type: "alert", text } as Modal, ...prev]);
		},
		prompt(text: string, cont: (text: string | null) => void) {
			const modal = {
				type: "prompt" as const,
				text,
				cont,
			};
			setModals((prev) => [modal as Modal, ...prev]);
		},
		confirm(text: string, cont: (confirmed: boolean) => void) {
			const modal = {
				type: "confirm" as const,
				text,
				cont,
			};
			setModals((prev) => [modal as Modal, ...prev]);
		},
	};

	// TEMP: for debugging
	(globalThis as typeof globalThis & { modalctl: typeof controller }).modalctl =
		controller;

	return (
		<ModalsContext.Provider value={[modals, controller]}>
			{p.children}
		</ModalsContext.Provider>
	);
};

export const useModals = (): ModalsContextType => {
	const context = useContext(ModalsContext);
	if (!context) {
		throw new Error("useModals must be used within a ModalsProvider");
	}
	return context;
};

export type ModalsContext2Type = {
	open: (modal: Modal) => void;
	alert: (text: string) => void;
	prompt: (text: string) => Promise<string | null>;
	confirm: (text: string) => Promise<boolean>;
	modals: Modal[];
};

export const useModals2 = (): ModalsContext2Type => {
	const context = useContext(ModalsContext);
	if (!context) {
		throw new Error("useModals2 must be used within a ModalsProvider");
	}

	return {
		...context[1],
		get modals() {
			return context[0];
		},
		alert(text: string) {
			context[1].alert(text);
		},
		prompt(text: string): Promise<string | null> {
			return new Promise((resolve) => {
				context[1].prompt(text, resolve);
			});
		},
		confirm(text: string): Promise<boolean> {
			return new Promise((resolve) => {
				context[1].confirm(text, resolve);
			});
		},
	};
};

export const useSettingsModals = () => {
	const location = useLocation();
	const nav = useNavigate();
	const [modals, modalCtl] = useModals();

	const openSettings = (modal: Modal) => {
		const lastModal = modals.at(-1);
		const isSettingsOpen = lastModal?.type === "user_settings";
		if (isSettingsOpen) {
			modalCtl.replace(modal);
		} else {
			modalCtl.open(modal);
		}
	};

	createEffect(
		on(
			() => location.pathname,
			(path) => {
				const userMatch = path.match(/^\/settings(\/([^/]+))?/);
				if (userMatch) {
					const [, , page] = userMatch;
					openSettings({
						type: "user_settings",
						page,
					});
				}

				const roomMatch = path.match(/^\/room\/([^/]+)\/settings(\/([^/]+))?/);
				if (roomMatch) {
					const [, room_id, , page] = roomMatch;
					openSettings({
						type: "room_settings",
						room_id,
						page,
					});
					// nav(`/room/${room_id}`, { replace: true });
				}

				const channelMatch = path.match(
					/^\/channel\/([^/]+)\/settings(\/([^/]+))?/,
				);
				if (channelMatch) {
					const [, channel_id, , page] = channelMatch;
					openSettings({
						type: "channel_settings",
						channel_id,
						page,
					});
					// nav(`/channel/${channel_id}`, { replace: true });
				}
			},
		),
	);
};
