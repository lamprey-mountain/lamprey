import { batch as solidBatch } from "solid-js";
import { createUpload } from "sdk";
import { handleSubmit } from "./submit.ts";
import type { Api } from "../api.tsx";
import type { ChatCtx } from "../context.ts";
import type { SetStoreFunction } from "solid-js/store";
import { Action, Data, Middleware } from "./types";
import { threadMarkRead } from "./middleware/threadMarkRead";
import { serverInitSession } from "./middleware/serverInitSession";
import { uploadCancel } from "./middleware/uploadCancel";
import { uploadInit } from "./middleware/uploadInit";
import { uploadPause } from "./middleware/uploadPause";
import { uploadResume } from "./middleware/uploadResume";

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
			case "menu.preview": {
				update("cursor", "preview", action.id);
				break;
			}
			case "modal.open":
				// This should be handled by the modal provider directly now
				console.warn(
					"modal.open should be replaced with useModals hook directly",
				);
				break;
			case "modal.close":
				// This should be handled by the modal provider directly now
				console.warn(
					"modal.close should be replaced with useModals hook directly",
				);
				break;
			case "modal.alert":
				// This should be handled by the modal provider directly now
				console.warn(
					"modal.alert should be replaced with useModals hook directly",
				);
				break;
			case "modal.prompt":
				// This should be handled by the modal provider directly now
				console.warn(
					"modal.prompt should be replaced with useModals hook directly",
				);
				break;
			case "modal.confirm":
				// This should be handled by the modal provider directly now
				console.warn(
					"modal.confirm should be replaced with useModals hook directly",
				);
				break;
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
		],
		ctx,
		api,
	);

	return d;
}
