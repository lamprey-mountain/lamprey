import { batch as solidBatch } from "solid-js";
import { createUpload } from "sdk";
import { handleSubmit } from "./submit.ts";
import type { Api } from "../api.tsx";
import type { ChatCtx } from "../context.ts";
import type { SetStoreFunction } from "solid-js/store";
import { Action, Data, Middleware, Modal } from "./types";
import { threadMarkRead } from "./middleware/threadMarkRead";
import { serverInitSession } from "./middleware/serverInitSession";
import { uploadCancel } from "./middleware/uploadCancel";
import { uploadInit } from "./middleware/uploadInit";
import { uploadPause } from "./middleware/uploadPause";
import { uploadResume } from "./middleware/uploadResume";
import { threadSend } from "./middleware/threadSend";

function combine(
	state: Data,
	update: SetStoreFunction<Data>,
	middleware: Array<Middleware>,
	ctx: ChatCtx,
	api: Api,
) {
	let _dispatch = (_action: Action) => {};
	const dispatch = (action: Action) => {
		switch (action.do) {
			case "modal.close": {
				update("modals", (modals) => modals.slice(1));
				break;
			}
			case "modal.open": {
				update("modals", (modals) => [...modals, action.modal]);
				break;
			}
			case "modal.alert": {
				update(
					"modals",
					(modals) => [{ type: "alert", text: action.text }, ...modals],
				);
				break;
			}
			case "modal.prompt": {
				const modal = {
					type: "prompt" as const,
					text: action.text,
					cont: action.cont,
				};
				update("modals", (modals) => [modal, ...modals]);
				break;
			}
			case "modal.confirm": {
				const modal = {
					type: "confirm" as const,
					text: action.text,
					cont: action.cont,
				};
				update("modals", (modals) => [modal, ...modals]);
				break;
			}
			case "menu.preview": {
				update("cursor", "preview", action.id);
				break;
			}
			default:
				// If no specific handling, assume it's a middleware-handled action
				// or an action that doesn't directly modify the main state via this dispatch.
				// Middleware will handle these.
				break;
		}
	};
	const merged = middleware.toReversed().reduce(
		(dispatch, m) => m(ctx, api, update)(dispatch),
		dispatch,
	);
	_dispatch = merged;
	return merged;
}

export function createDispatcher(
	ctx: ChatCtx,
	api: Api,
	update: SetStoreFunction<Data>,
) {
	const d = combine(
		ctx.data,
		update,
		[
			threadMarkRead,
			serverInitSession,
			uploadCancel,
			uploadInit,
			uploadPause,
			uploadResume,
			threadSend,
		],
		ctx,
		api,
	);

	return d;
}
