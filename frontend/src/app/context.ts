import type { Placement } from "@floating-ui/dom";
import type { Emitter } from "@solid-primitives/event-bus";
import type * as i18n from "@solid-primitives/i18n";
import type { ReactiveMap } from "@solid-primitives/map";
import type {
	Client,
	Media,
	MessageReady,
	MessageSearch,
	MessageSync,
	Preferences,
	Upload,
} from "sdk";
import {
	type Accessor,
	createContext,
	type Setter,
	useContext,
} from "solid-js";
import type { SetStoreFunction } from "solid-js/store";
import type { ChannelContextT } from "@/contexts/channel";
import type { DocumentContextT } from "@/contexts/document.tsx";
import type { RoomContextT } from "@/contexts/room.tsx";
import type { SlashCommands } from "@/contexts/slash-commands";
import type en from "@/i18n/en.tsx";

// Re-export context types
export type { Menu } from "@/contexts/menu.tsx";
export type { Modal } from "@/contexts/modal.tsx";
// Re-export all chat types from the types module
export type {
	Attachment,
	AttachmentCreateT,
	ChannelSearch,
	ChatCtx,
	Cursor,
	CursorStats,
	Data,
	Events,
	MediaCtx,
	Popout,
	Slice,
	ThreadsViewData,
} from "@/types/chat";

// Runtime context creation
export const chatctx = createContext<import("@/types/chat").ChatCtx>();
export const useCtx = (): import("@/types/chat").ChatCtx => {
	const ctx = useContext(chatctx);
	if (!ctx) {
		throw new Error("useCtx must be used within a ChatCtx provider");
	}
	return ctx;
};
