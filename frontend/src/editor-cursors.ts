import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import {
	autoUpdate,
	computePosition,
	flip,
	offset,
	shift,
} from "@floating-ui/dom";
import { Api } from "./api.tsx";
import { getColor } from "./colors.ts";
import { base64UrlDecode, base64UrlEncode } from "./editor-utils.ts";
import * as Y from "yjs";
import {
	absolutePositionToRelativePosition,
	relativePositionToAbsolutePosition,
	ySyncPluginKey,
} from "y-prosemirror";

const cursorPluginKey = new PluginKey("cursorPlugin");

export const cursorPlugin = (api: Api, channelId: string, branchId: string) => {
	return new Plugin({
		key: cursorPluginKey,
		state: {
			init() {
				return { cursors: new Map() };
			},
			apply(tr, value) {
				const meta = tr.getMeta(cursorPluginKey);
				if (meta) {
					const newCursors = new Map(value.cursors);
					if (meta.type === "update") {
						newCursors.set(meta.userId, {
							name: meta.name,
							color: meta.color,
							cursor: meta.cursor,
						});
					} else if (meta.type === "remove") {
						newCursors.delete(meta.userId);
					}
					return { cursors: newCursors };
				}
				return value;
			},
		},
		props: {
			decorations(state) {
				const { cursors } = cursorPluginKey.getState(state);
				const decos: Decoration[] = [];
				const yState = ySyncPluginKey.getState(state);

				if (!yState || !yState.binding) return DecorationSet.empty;

				const selfId = api.users.cache.get("@self")?.id;

				for (const [userId, data] of cursors) {
					if (userId === selfId) continue;

					const anchor = relativePositionToAbsolutePosition(
						yState.doc,
						yState.type,
						Y.createRelativePositionFromJSON(data.cursor.anchor),
						yState.binding.mapping,
					);
					const head = relativePositionToAbsolutePosition(
						yState.doc,
						yState.type,
						Y.createRelativePositionFromJSON(data.cursor.head),
						yState.binding.mapping,
					);

					if (anchor === null || head === null) continue;

					// render selections
					const from = Math.min(anchor, head);
					const to = Math.max(anchor, head);

					if (from !== to) {
						decos.push(Decoration.inline(from, to, {
							style:
								`background-color: color-mix(in srgb, ${data.color}, transparent 70%)`,
						}));
					}

					// render cursors
					decos.push(Decoration.widget(head, (view) => {
						const widget = document.createElement("span");
						widget.classList.add("document-presence-cursor");
						widget.style.borderLeft = `2px solid ${data.color}`;

						const label = document.createElement("div");
						label.classList.add("document-presence-name");
						label.textContent = data.name;
						label.style.backgroundColor = data.color;

						widget.appendChild(label);

						const cleanup = autoUpdate(widget, label, () => {
							computePosition(widget, label, {
								placement: "top-start",
								middleware: [
									offset(4),
									flip(),
									shift({ padding: 4 }),
								],
							}).then(({ x, y }) => {
								label.style.translate = `${x}px ${y}px`;
							});
						});

						// FIXME: there may be multiple editors
						(widget as any)._floating_cleanup = cleanup;

						return widget;
					}, {
						key: userId,
						destroy(dom) {
							if ((dom as any)._floating_cleanup) {
								(dom as any)._floating_cleanup();
							}
						},
					}));
				}

				return DecorationSet.create(state.doc, decos);
			},
		},
		view(view) {
			const onSync = (msg: any) => {
				if (
					msg.type === "DocumentPresence" && msg.channel_id === channelId &&
					msg.branch_id === branchId
				) {
					const selfId = api.users.cache.get("@self")?.id;
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

					// FIXME: use room_member.override_name if it exists
					// FIXME: update name live
					const user = api.users.cache.get(msg.user_id);
					const name = user?.name || "Unknown";
					const color = getColor(msg.user_id);

					const tr = view.state.tr;
					tr.setMeta(cursorPluginKey, {
						type: cursor ? "update" : "remove",
						userId: msg.user_id,
						name,
						color,
						cursor,
					});
					view.dispatch(tr);
				}
			};

			api.events.on("sync", onSync);

			return {
				update(view, prevState) {
					const yState = ySyncPluginKey.getState(view.state);
					if (!yState || !yState.binding) return;

					const sel = view.state.selection;
					const prevSel = prevState.selection;

					if (!sel.eq(prevSel)) {
						const { anchor, head } = sel;
						const anchorRel = absolutePositionToRelativePosition(
							anchor,
							yState.type,
							yState.binding.mapping,
						);
						const headRel = absolutePositionToRelativePosition(
							head,
							yState.type,
							yState.binding.mapping,
						);

						const anchorEnc = base64UrlEncode(
							Y.encodeRelativePosition(anchorRel),
						);
						const headEnc = base64UrlEncode(Y.encodeRelativePosition(headRel));

						const ws = api.client.getWebsocket();
						if (ws.readyState === WebSocket.OPEN) {
							ws.send(JSON.stringify({
								type: "DocumentPresence",
								channel_id: channelId,
								branch_id: branchId,
								cursor_head: headEnc,
								cursor_tail: anchorEnc,
							}));
						}
					}
				},
				destroy() {
					// NOTE: api.events is a solidjs emitter, which doesn't have .off
					// it's supposed to be automatically cleaned up, but i'm not sure if it actually works here?
					// api.events.off("sync", onSync);
				},
			};
		},
	});
};
