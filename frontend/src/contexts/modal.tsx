import type { Media, Tag } from "sdk";
import { createContext, type ParentProps, useContext } from "solid-js";
import { createStore } from "solid-js/store";

export type Modal =
	| { type: "alert"; text: string }
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
			cont: (
				data: { name: string; type: "Text" | "Voice" | "Category" } | null,
			) => void;
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
			editor: any;
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
			type: "room_create";
			cont: (data: { name: string; public: boolean } | null) => void;
	  };

export type ModalsController = {
	close: () => void;
	open: (modal: Modal) => void;
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
	(globalThis as any).modalctl = controller;

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
