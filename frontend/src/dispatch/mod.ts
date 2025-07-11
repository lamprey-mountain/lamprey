import { batch as solidBatch } from "solid-js";
import { createUpload } from "sdk";
import { handleSubmit } from "./submit.ts";
import type { Api } from "../api.tsx";
import type { ChatCtx } from "../context.ts";
import type { SetStoreFunction } from "solid-js/store";
import { Action, Data, Middleware, reduce, Reduction } from "./types";
import { log } from "./middleware/log";
import { threadMarkRead } from "./middleware/threadMarkRead";
import { serverInitSession } from "./middleware/serverInitSession";
import { uploadCancel } from "./middleware/uploadCancel";
import { uploadInit } from "./middleware/uploadInit";
import { uploadPause } from "./middleware/uploadPause";
import { uploadResume } from "./middleware/uploadResume";
import { mouseMoved } from "./middleware/mouseMoved";
import { threadSend } from "./middleware/threadSend";

function combine(
	reduce: (state: Data, delta: Reduction) => Data,
	state: Data,
	update: SetStoreFunction<Data>,
	middleware: Array<Middleware>,
) {
	let _dispatch = (_action: Action) => {};
	const dispatch = (action: Action) => {
		console.log("reduce", state, action);
		update((s) => reduce(s, action as Reduction));
	};
	const merged = middleware.toReversed().reduce(
		(dispatch, m) => m(dispatch),
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
		reduce,
		ctx.data,
		update,
		[
			log,
			threadMarkRead,
			serverInitSession,
			uploadCancel,
			uploadInit,
			uploadPause,
			uploadResume,
			mouseMoved,
			threadSend,
		],
		ctx,
		api,
	);

	return d;
}
