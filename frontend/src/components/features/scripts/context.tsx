import { debounce } from "@solid-primitives/scheduled";
import { ReactiveSet } from "@solid-primitives/set";
import type { MessageSync, Script } from "sdk";
import { createContext, createSignal, onCleanup, useContext } from "solid-js";
import * as Y from "yjs";
import { useApi } from "@/api";
import type { PaneNode } from "@/components/panes/context";
import { base64UrlDecode, base64UrlEncode } from "../editor/editor-utils";

type ScriptContextT = {
	channel_id: string;
	documents: Map<string, Y.Doc>;
	acquire(redex: Script): Y.Doc | null;
	isSubscribed(id: string): boolean;
};

export type ScriptPane = PaneNode<ScriptPaneType>;

export type ScriptPaneType =
	| { type: "script_code"; script_id: string }
	| { type: "script_inputs"; script_id: string }
	| { type: "script_preview"; script_id: string }
	| { type: "run_logs"; script_id: string; run_id: string };

export const ScriptContext = createContext<ScriptContextT>();

export const createScriptContext = (channel_id: string) => {
	const api = useApi();
	const activeSubscriptions = new Map<string, number>();
	const subscribedDocs = new ReactiveSet<string>();

	const scheduleSubscribe = debounce(() => {
		const documents = Array.from(ctx.documents.entries()).map(([id, doc]) => ({
			channel_id,
			redex_id: id,
			branch_id: id,
			state_vector: base64UrlEncode(Y.encodeStateVector(doc)),
		}));

		api.client.send({
			type: "Subscribe",
			documents,
		});
	}, 0);

	const ctx: ScriptContextT = {
		channel_id,
		documents: new Map(),
		acquire(redex: Script) {
			if (redex.latest_version.location.type !== "Document") {
				return null;
			}

			const id = redex.id;
			const currentCount = activeSubscriptions.get(id) ?? 0;
			activeSubscriptions.set(id, currentCount + 1);

			onCleanup(() => {
				const count = activeSubscriptions.get(id) ?? 0;
				if (count <= 1) {
					activeSubscriptions.delete(id);
					ctx.documents.delete(id);
					subscribedDocs.delete(id);
					scheduleSubscribe();
				} else {
					activeSubscriptions.set(id, count - 1);
				}
			});

			const existing = ctx.documents.get(id);
			if (existing) return existing;

			const ydoc = new Y.Doc();
			ydoc.on("update", (update, origin) => {
				if (origin && origin.key === "server") return;

				api.client.send({
					type: "DocumentEdit",
					channel_id: channel_id,
					branch_id: id,
					redex_id: id,
					update: base64UrlEncode(update),
				});
			});

			ctx.documents.set(id, ydoc);
			scheduleSubscribe();

			return ydoc;
		},
		isSubscribed(id: string) {
			return subscribedDocs.has(id);
		},
	};

	api.events.on("sync", ([msg]: [MessageSync, unknown]) => {
		if (msg.type === "DocumentEdit") {
			// TODO(?): create a new ydoc if it doesnt exist yet
			if (!ctx.documents.has(msg.document_id)) return;
			const ydoc = ctx.documents.get(msg.document_id)!;
			const update = (
				(msg.update as unknown) instanceof Uint8Array
					? msg.update
					: base64UrlDecode(msg.update as unknown as string)
			) as Uint8Array;
			Y.applyUpdate(ydoc, update, { key: "server" });
		} else if (msg.type === "DocumentSubscribed") {
			if (ctx.documents.has(msg.document_id)) {
				subscribedDocs.add(msg.document_id);
			}
		} else if (msg.type === "DocumentPresence") {
			// TODO
		}
	});

	return ctx;
};

export const useScript = () => {
	const ctx = useContext(ScriptContext);
	if (!ctx) {
		throw new Error("useScript must be used within a ScriptContext.Provider");
	}
	return ctx;
};
