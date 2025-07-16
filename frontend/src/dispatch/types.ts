import { type Api } from "../api.tsx";
import { type ChatCtx, type Data } from "../context.ts";
import { type SetStoreFunction } from "solid-js/store";




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
};

export type Action =
	| ModalAction
	| ServerAction
	| ThreadAction
	| UploadAction
	| WindowAction;

export type ModalAction =
	| { do: "modal.open"; modal: Modal }
	| { do: "modal.close" }
	| { do: "modal.alert"; text: string }
	| { do: "modal.prompt"; text: string; cont: (text: string | null) => void }
	| { do: "modal.confirm"; text: string; cont: (confirmed: boolean) => void };

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

export type UploadAction =
	| { do: "upload.init"; local_id: string; thread_id: string; file: File }
	| { do: "upload.pause"; local_id: string }
	| { do: "upload.resume"; local_id: string }
	| { do: "upload.cancel"; local_id: string; thread_id: string };

export type WindowAction = { do: "window.mouse_move"; e: MouseEvent };

export type Dispatcher = (action: Action) => void;
