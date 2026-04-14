import { createContext, useContext } from "solid-js";

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
