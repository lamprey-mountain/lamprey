import { type Api } from "../api.tsx";
import { type ChatCtx, type Data } from "../context.ts";
import { type SetStoreFunction } from "solid-js/store";
export { type Data } from "../context.ts";

export type Middleware = (
	ctx: ChatCtx,
	api: Api,
	update: SetStoreFunction<Data>,
) => (next: (action: Action) => void) => (action: Action) => void;

export type Modal = {
	type: "settings";
	user_id: string;
	page?: string;
} | {
	type: "room_settings";
	room_id: string;
	page?: string;
} | {
	type: "thread_settings";
	thread_id: string;
	page?: string;
} | {
	type: "alert";
	text: string;
} | {
	type: "prompt";
	text: string;
	cont: (text: string | null) => void;
} | {
	type: "confirm";
	text: string;
	cont: (confirmed: boolean) => void;
} | {
	type: "message_edits";
	channel_id: string;
	message_id: string;
};

export type Action =
	| ServerAction
	| ThreadAction
	| CategoryAction
	| UploadAction
	| WindowAction
	| { do: "menu.preview"; id: string | null };

export type ServerAction =
	| { do: "server.init_session" }
	| { do: "server.login"; token: string }
	| { do: "server.logout" };

export type ThreadAction =
	| {
		do: "thread.mark_read";
		thread_id: string;
		version_id: string;
		delay: boolean;
		also_local: boolean;
	}
	| { do: "thread.send"; thread_id: string; text: string };

export type CategoryAction = {
	do: "category.mark_read";
	category_id: string;
};

export type WindowAction = { do: "window.mouse_move"; e: MouseEvent };

export type Dispatcher = (action: Action) => void;
