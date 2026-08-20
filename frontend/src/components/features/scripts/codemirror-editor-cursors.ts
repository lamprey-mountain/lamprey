import { StateEffect, StateField } from "@codemirror/state";
import {
	Decoration,
	type DecorationSet,
	type EditorView,
	ViewPlugin,
	type ViewUpdate,
	WidgetType,
} from "@codemirror/view";
import {
	autoUpdate,
	computePosition,
	flip,
	offset,
	shift,
} from "@floating-ui/dom";
import type { MessageSync } from "sdk";
import * as Y from "yjs";
import type { Api } from "@/api";
import { getColor } from "@/lib/colors";
import { base64UrlDecode, base64UrlEncode } from "../editor/editor-utils";

type CursorData = {
	name: string;
	color: string;
	cursor: { head: any; anchor: any };
};

class CursorWidget extends WidgetType {
	constructor(
		public readonly color: string,
		public readonly name: string,
	) {
		super();
	}

	eq(other: CursorWidget) {
		return other.color === this.color && other.name === this.name;
	}

	toDOM() {
		const widget = document.createElement("span");
		widget.classList.add("document-presence-cursor");
		widget.style.borderLeft = `2px solid ${this.color}`;

		const label = document.createElement("div");
		label.classList.add("document-presence-name");
		label.textContent = this.name;
		label.style.backgroundColor = this.color;

		widget.appendChild(label);

		const cleanup = autoUpdate(widget, label, () => {
			computePosition(widget, label, {
				placement: "top-start",
				middleware: [offset(4), flip(), shift({ padding: 4 })],
			}).then(({ x, y }) => {
				label.style.translate = `${x}px ${y}px`;
			});
		});

		(widget as any)._floating_cleanup = cleanup;

		return widget;
	}

	destroy(dom: HTMLElement) {
		const cleanup = (dom as any)._floating_cleanup;
		if (cleanup) cleanup();
	}
}

export const cursorPlugin = (
	api: Api,
	channelId: string,
	redexId: string,
	ytext: Y.Text,
) => {
	const cursorEffect = StateEffect.define<{
		type: "update" | "remove";
		userId: string;
		name: string;
		color: string;
		cursor: any;
	}>();

	const cursorState = StateField.define<Map<string, CursorData>>({
		create() {
			return new Map();
		},
		update(cursors, tr) {
			let changed = false;
			const newCursors = new Map(cursors);
			for (const effect of tr.effects) {
				if (effect.is(cursorEffect)) {
					changed = true;
					if (effect.value.type === "update") {
						newCursors.set(effect.value.userId, {
							name: effect.value.name,
							color: effect.value.color,
							cursor: effect.value.cursor,
						});
					} else {
						newCursors.delete(effect.value.userId);
					}
				}
			}
			return changed ? newCursors : cursors;
		},
	});

	const cursorDecorations = ViewPlugin.fromClass(
		class {
			decorations: DecorationSet;
			unsubscribe: () => void;

			constructor(view: EditorView) {
				this.decorations = this.buildDecorations(view);

				const onSync = (payload: [MessageSync, unknown]) => {
					const [msg] = payload;
					if (
						msg.type === "DocumentPresence" &&
						msg.channel_id === channelId &&
						msg.branch_id === redexId
					) {
						const currentUser = api.users.cache.get("@self");
						const selfId = currentUser?.id;
						if (msg.user_id === selfId) return;

						let cursor = null;
						if (msg.cursor_head) {
							try {
								const head = Y.decodeRelativePosition(
									base64UrlDecode(msg.cursor_head),
								);
								const anchor = msg.cursor_tail
									? Y.decodeRelativePosition(base64UrlDecode(msg.cursor_tail))
									: head;
								cursor = {
									head: Y.relativePositionToJSON(head),
									anchor: Y.relativePositionToJSON(anchor),
								};
							} catch (e) {
								console.error("failed to decode cursor", e);
							}
						}

						const user = api.users.cache.get(msg.user_id);
						const name = user?.name || "Unknown";
						const color = getColor(msg.user_id);

						view.dispatch({
							effects: cursorEffect.of({
								type: cursor ? "update" : "remove",
								userId: msg.user_id,
								name,
								color,
								cursor,
							}),
						});
					}
				};

				this.unsubscribe = api.events.on("sync", onSync);
			}

			update(update: ViewUpdate) {
				const cursors = update.state.field(cursorState);
				const prevCursors = update.startState.field(cursorState);

				if (cursors !== prevCursors || update.docChanged) {
					this.decorations = this.buildDecorations(update.view);
				}

				if (update.selectionSet) {
					this.sendPresence(update.view);
				}
			}

			sendPresence(view: EditorView) {
				const sel = view.state.selection.main;

				const headRel = Y.createRelativePositionFromTypeIndex(ytext, sel.head);
				const anchorRel = Y.createRelativePositionFromTypeIndex(
					ytext,
					sel.anchor,
				);

				const anchorEnc = base64UrlEncode(Y.encodeRelativePosition(anchorRel));
				const headEnc = base64UrlEncode(Y.encodeRelativePosition(headRel));

				api.client.send({
					type: "DocumentPresence",
					channel_id: channelId,
					branch_id: redexId,
					redex_id: redexId,
					cursor_head: headEnc,
					cursor_tail: anchorEnc,
				});
			}

			buildDecorations(view: EditorView): DecorationSet {
				const cursors = view.state.field(cursorState);
				const decos = [];
				const currentUser = api.users.cache.get("@self");
				const selfId = currentUser?.id;

				for (const [userId, data] of cursors) {
					if (userId === selfId) continue;

					if (!data.cursor) continue;

					try {
						const headRel = Y.createRelativePositionFromJSON(data.cursor.head);
						const anchorRel = Y.createRelativePositionFromJSON(
							data.cursor.anchor,
						);

						const headAbs = Y.createAbsolutePositionFromRelativePosition(
							headRel,
							ytext.doc!,
						);
						const anchorAbs = Y.createAbsolutePositionFromRelativePosition(
							anchorRel,
							ytext.doc!,
						);

						if (!headAbs || !anchorAbs) continue;

						const head = headAbs.index;
						const anchor = anchorAbs.index;

						const from = Math.min(anchor, head);
						const to = Math.max(anchor, head);

						const docLength = view.state.doc.length;

						if (from < 0 || to > docLength) continue;

						if (from !== to) {
							decos.push(
								Decoration.mark({
									class: "document-presence-selection",
									attributes: {
										style: `background-color: color-mix(in srgb, ${data.color}, transparent 70%)`,
									},
								}).range(from, to),
							);
						}

						decos.push(
							Decoration.widget({
								widget: new CursorWidget(data.color, data.name),
								side: 1,
							}).range(head),
						);
					} catch (e) {
						console.error("error building decoration for cursor", e);
					}
				}

				return Decoration.set(decos, true);
			}

			destroy() {
				this.unsubscribe();
			}
		},
		{
			decorations: (v) => v.decorations,
		},
	);

	return [cursorState, cursorDecorations];
};
