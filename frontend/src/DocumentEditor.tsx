import * as Y from "yjs";
import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { DOMParser, Schema } from "prosemirror-model";
import { ySyncPlugin, yCursorPlugin, yUndoPlugin, undo, redo, initProseMirrorDoc } from 'y-prosemirror'
// import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { createEffect, onCleanup, onMount } from "solid-js";
import { initTurndownService } from "./turndown.ts";
import { decorate, md } from "./markdown.tsx";
import { useCtx } from "./context";
import { createWrapCommand } from "./editor-utils.ts";
import { Observable } from "lib0/observable";
import { useApi } from "./api.tsx";

const turndown = initTurndownService();

const schema = new Schema({
  nodes: {
    doc: {
      content: "block+",
    },
    paragraph: {
      content: "inline*",
      group: "block",
      whitespace: "pre",
      toDOM: () => ["p", 0],
      parseDOM: ["p", "x-html-import"].map((tag) => ({
        tag,
        preserveWhitespace: "full",
      })),
    },
    // maybe have special purpose blocks instead of pure markdown (markdown with incremental rich text)
    // blockquote: {},
    // table: {},
    // codeblock: {},
    // details: {},
    // media: {},
    mention: {
      group: "inline",
      atom: true,
      inline: true,
      selectable: false,
      attrs: {
        user: {},
      },
      leafText(node) {
        return `<@${node.attrs.user}>`;
      },
      toDOM: (
        n,
      ) => ["span", { "data-user-id": n.attrs.user, "class": "mention" }],
      parseDOM: [{
        tag: "span.mention[data-user-id]",
        getAttrs: (el) => ({ user: (el as HTMLElement).dataset.userId }),
      }],
    },
    mentionChannel: {
      group: "inline",
      atom: true,
      inline: true,
      selectable: false,
      attrs: {
        channel: {},
      },
      leafText(node) {
        return `<#${node.attrs.channel}>`;
      },
      toDOM: (
        n,
      ) => ["span", { "data-channel-id": n.attrs.channel, "class": "mention" }],
      parseDOM: [{
        tag: "span.mention[data-channel-id]",
        getAttrs: (el) => ({ channel: (el as HTMLElement).dataset.channelId }),
      }],
    },
    emoji: {
      group: "inline",
      atom: true,
      inline: true,
      selectable: false,
      attrs: {
        id: {},
        name: {},
      },
      leafText(node) {
        return `<:${node.attrs.name}:${node.attrs.id}>`;
      },
      toDOM: (
        n,
      ) => ["span", {
        "data-emoji-id": n.attrs.id,
        "data-emoji-name": n.attrs.name,
      }, `:${n.attrs.name}:`],
      parseDOM: [{
        tag: "span[data-emoji-id][data-emoji-name]",
        getAttrs: (el) => ({
          id: (el as HTMLElement).dataset.emojiId,
          name: (el as HTMLElement).dataset.emojiName,
        }),
      }],
    },
    text: {
      group: "inline",
      inline: true,
    },
  },
});

type EditorProps = {
  initialContent?: string;
  keymap?: { [key: string]: Command };
  initialSelection?: "start" | "end";
  mentionRenderer?: (node: HTMLElement, userId: string) => void;
  mentionChannelRenderer?: (node: HTMLElement, channelId: string) => void;
};

type EditorViewProps = {
  placeholder?: string;
  disabled?: boolean;
  onUpload?: (file: File) => void;
  onSubmit: (text: string) => boolean;
  onChange?: (state: EditorState) => void;
  channelId?: string; // Needed for autocomplete
  submitOnEnter?: boolean;
};

// TODO: move lamprey provider code here
class LampreyProvider extends Observable {
  constructor(ydoc: Y.Doc) {
    super()

    ydoc.on('update', (update, origin) => {
      // ignore updates applied by this provider
      if (origin !== this) {
        // this update was produced either locally or by another provider.
        this.emit('update', [update])
      }
    })

    // listen to an event that fires when a remote update is received
    this.on('update', update => {
      Y.applyUpdate(ydoc, update, this) // the third parameter sets the transaction-origin
    })
  }
}

/** decode unpadded url safe base64 */
function base64UrlDecode(str: string): Uint8Array {
  str = str.replace(/-/g, '+').replace(/_/g, '/');

  const pad = str.length % 4;
  if (pad) {
    str += '='.repeat(4 - pad);
  }

  const binary = atob(str);
  const bytes = new Uint8Array(binary.length);

  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return bytes;
}

function base64UrlEncode(bytes: Uint8Array): string {
  let binary = "";
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

export const createEditor = (
  opts: EditorProps,
  channelId: string,
  branchId: string,
) => {
  const ctx = useCtx();
  const api = useApi();

  const ydoc = new Y.Doc();
  const type = ydoc.get("prosemirror", Y.XmlFragment);
  const { doc, mapping } = initProseMirrorDoc(type, schema);

  const onSync = (msg: any) => {
    if (msg.type === "DocumentEdit") {
      if (msg.channel_id === channelId && msg.branch_id === branchId) {
        const update = base64UrlDecode(msg.update);
        Y.applyUpdate(ydoc, update);
      }
    }
  };

  api.events.on("sync", onSync);

  const subscribe = () => {
    const ws = api.client.getWebsocket();
    if (ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({
      type: "DocumentSubscribe",
      channel_id: channelId,
      branch_id: branchId,
      state_vector: base64UrlEncode(Y.encodeStateVector(ydoc)),
    }));
  };

  subscribe();

  ydoc.on("update", (update, origin) => {
    if (origin === provider) return;
    const ws = api.client.getWebsocket();
    if (ws.readyState !== WebSocket.OPEN) return;

    ws.send(JSON.stringify({
      type: "DocumentEdit",
      channel_id: channelId,
      branch_id: branchId,
      update: base64UrlEncode(update),
    }));
  });

  const provider = new LampreyProvider(ydoc);

  let editorRef!: HTMLDivElement;
  let view: EditorView | undefined;
  let onSubmit!: (content: string) => boolean | undefined;
  let submitOnEnter = true;

  const submitCommand: Command = (state, dispatch) => {
    const shouldClear = onSubmit?.(state.doc.textContent.trim());
    if (shouldClear) {
      dispatch?.(state.tr.deleteRange(0, state.doc.nodeSize - 2));
    }
    return true;
  };

  const createState = () => {
    let doc;
    if (opts.initialContent) {
      const div = document.createElement("div");
      div.innerHTML = md.parser(md.lexer(opts.initialContent));
      doc = DOMParser.fromSchema(schema).parse(div);
    }

    let selection;
    if (doc && opts.initialSelection) {
      let pos = 1;
      if (opts.initialSelection === "end") {
        pos = doc.content.size - 1;
      }
      selection = TextSelection.create(doc, pos);
    }

    return EditorState.create({
      doc,
      selection,
      schema,
      plugins: [
        ySyncPlugin(type, { mapping }),
        yUndoPlugin(),
        keymap({
          "Ctrl-z": undo,
          "Ctrl-Shift-z": redo,
          "Ctrl-y": redo,
          "Ctrl-b": createWrapCommand("**"),
          "Ctrl-i": createWrapCommand("*"),
          "Ctrl-`": createWrapCommand("`"),
          "Ctrl-m": (_state) => {
            return false;
          },
          "Shift-Enter": (state, dispatch) => {
            dispatch?.(state.tr.insertText("\n"));
            return true;
          },
          "Ctrl-Enter": submitCommand,
          "Enter": (state, dispatch) => {
            if (submitOnEnter) {
              return submitCommand(state, dispatch);
            }
            dispatch?.(state.tr.insertText("\n"));
            return true;
          },
          "Backspace": (state, dispatch) => {
            const sel = state.tr.selection;
            if (sel.empty) {
              const pos = sel.$anchor.pos - 1;
              if (pos >= 0) {
                dispatch?.(state.tr.deleteRange(pos, pos + 1));
              }
            } else {
              dispatch?.(state.tr.deleteSelection());
            }
            return true;
          },
          ...opts.keymap,
        }),
      ],
    });
  };

  return {
    setState(state?: EditorState) {
      view?.updateState(state ?? createState());
    },
    focus() {
      view?.focus();
    },
    View(props: EditorViewProps) {
      createEffect(() => {
        onSubmit = props.onSubmit;
        submitOnEnter = props.submitOnEnter ?? true;
      });

      onMount(() => {
        const ctx = useCtx(); // Access context inside mount since we're in a Solid component

        view = new EditorView(editorRef!, {
          domParser: DOMParser.fromSchema(schema),
          state: createState(),
          decorations(state) {
            return decorate(state, props.placeholder);
          },
          nodeViews: {
            mention: (node) => {
              const dom = document.createElement("span");
              dom.classList.add("mention");
              if (opts.mentionRenderer) {
                opts.mentionRenderer(dom, node.attrs.user);
              } else {
                dom.textContent = `@${node.attrs.user}`;
              }
              return { dom };
            },
            mentionChannel: (node) => {
              const dom = document.createElement("span");
              dom.classList.add("mention");
              if (opts.mentionChannelRenderer) {
                opts.mentionChannelRenderer(dom, node.attrs.channel);
              } else {
                dom.textContent = `#${node.attrs.channel}`;
              }
              return { dom };
            },
          },
          handlePaste(view, event, slice) {
            const files = Array.from(event.clipboardData?.files ?? []);
            if (files.length) {
              for (const file of files) props.onUpload?.(file);
              return true;
            }
            const str = slice.content.textBetween(0, slice.size);
            const tr = view.state.tr;
            if (
              /^(https?:\/\/|mailto:)\S+$/i.test(str) && !tr.selection.empty
            ) {
              tr.insertText("[", tr.selection.from);
              tr.insertText(`](${str})`, tr.selection.to);
              tr.setSelection(TextSelection.create(tr.doc, tr.selection.to));
              view.dispatch(
                tr.scrollIntoView().setMeta("paste", true).setMeta(
                  "uiEvent",
                  "paste",
                ),
              );
            } else {
              const textToParse = slice.content.textBetween(
                0,
                slice.content.size,
                "\n",
              );
              const div = document.createElement("div");
              div.innerHTML = md.parser(md.lexer(textToParse));
              const newSlice = DOMParser.fromSchema(schema).parseSlice(div);
              view.dispatch(
                view.state.tr.replaceSelection(newSlice).scrollIntoView()
                  .setMeta("paste", true),
              );
            }
            return true;
          },
          handleKeyDown(view, event) {
            // TODO: use actual css line height here
            const LINE_HEIGHT = 18;
            const refElement = () => {
              const cursorPos = view.coordsAtPos(view.state.selection.from);
              return {
                getBoundingClientRect() {
                  return {
                    x: cursorPos.left,
                    y: cursorPos.bottom - LINE_HEIGHT,
                    left: cursorPos.left,
                    right: cursorPos.right,
                    top: cursorPos.bottom - LINE_HEIGHT,
                    bottom: cursorPos.bottom,
                    width: 0,
                    height: LINE_HEIGHT,
                  };
                },
              };
            };

            if (event.key === "/") {
              const state = view.state;
              if (state.selection.from === 1) {
                ctx.setAutocomplete({
                  type: "command",
                  query: "",
                  ref: refElement() as any,
                  onSelect: (command: string) => {
                    const state = view.state;
                    const from = 0;
                    const to = state.selection.to;

                    let tr = state.tr.replaceWith(
                      from,
                      to,
                      schema.text(`/${command} `),
                    );
                    const posAfter = tr.mapping.map(to);
                    tr = tr.setSelection(
                      TextSelection.create(tr.doc, posAfter + 1),
                    );

                    view.dispatch(tr);
                    ctx.setAutocomplete(null);
                  },
                  channelId: props.channelId || "",
                });
              }

              return false;
            }

            // if the @ character was pressed, open the menu
            if (event.key === "@") {
              ctx.setAutocomplete({
                type: "mention",
                query: "",
                ref: refElement() as any,
                onSelect: (userId: string, _userName: string) => {
                  const state = view.state;
                  const from = Math.max(0, state.selection.from - 1);
                  const to = state.selection.to;

                  let mentionStart = from;
                  while (mentionStart > 0) {
                    const char = state.doc.textBetween(
                      mentionStart - 1,
                      mentionStart,
                    );
                    if (char === "@" || /\w/.test(char)) {
                      mentionStart--;
                    } else {
                      break;
                    }
                  }

                  let tr = state.tr.replaceWith(
                    mentionStart,
                    to,
                    schema.nodes.mention.create({ user: userId }),
                  );
                  const posAfter = tr.mapping.map(to);
                  tr = tr.insert(posAfter, schema.text(" ", []));
                  tr = tr.setSelection(
                    TextSelection.create(tr.doc, posAfter + 1),
                  );

                  view.dispatch(tr);

                  ctx.setAutocomplete(null);
                },
                channelId: props.channelId || "",
              });

              return false;
            }

            if (event.key === "#") {
              ctx.setAutocomplete({
                type: "channel",
                query: "",
                ref: refElement() as any,
                onSelect: (channelId: string, _channelName: string) => {
                  const state = view.state;
                  const from = Math.max(0, state.selection.from - 1);
                  const to = state.selection.to;

                  let mentionStart = from;
                  while (mentionStart > 0) {
                    const char = state.doc.textBetween(
                      mentionStart - 1,
                      mentionStart,
                    );
                    if (char === "#" || /\w/.test(char)) {
                      mentionStart--;
                    } else {
                      break;
                    }
                  }

                  let tr = state.tr.replaceWith(
                    mentionStart,
                    to,
                    schema.nodes.mentionChannel.create({ channel: channelId }),
                  );
                  const posAfter = tr.mapping.map(to);
                  tr = tr.insert(posAfter, schema.text(" ", []));
                  tr = tr.setSelection(
                    TextSelection.create(tr.doc, posAfter + 1),
                  );

                  view.dispatch(tr);

                  ctx.setAutocomplete(null);
                },
                channelId: props.channelId || "",
              });

              return false;
            }

            if (event.key === ":") {
              ctx.setAutocomplete({
                type: "emoji",
                query: "",
                ref: refElement() as any,
                onSelect: (id: string, name: string, char?: string) => {
                  const state = view.state;
                  const from = Math.max(0, state.selection.from - 1);
                  const to = state.selection.to;

                  let mentionStart = from;
                  while (mentionStart > 0) {
                    const char = state.doc.textBetween(
                      mentionStart - 1,
                      mentionStart,
                    );
                    if (char === ":" || /[\w_]/.test(char)) {
                      mentionStart--;
                    } else {
                      break;
                    }
                  }

                  let tr = state.tr;
                  if (char) { // unicode
                    tr = tr.replaceWith(mentionStart, to, schema.text(char));
                  } else { // custom
                    tr = tr.replaceWith(
                      mentionStart,
                      to,
                      schema.nodes.emoji.create({ id, name }),
                    );
                  }

                  const posAfter = tr.mapping.map(to);
                  tr = tr.insert(posAfter, schema.text(" ", []));
                  tr = tr.setSelection(
                    TextSelection.create(tr.doc, posAfter + 1),
                  );

                  view.dispatch(tr);
                  ctx.setAutocomplete(null);
                },
                channelId: props.channelId || "",
              });
              return false;
            }

            // autocomplete navigation and selection
            if (ctx?.autocomplete()) {
              if (
                event.key === "ArrowUp" || event.key === "ArrowDown" ||
                event.key === "Enter" || event.key === "Tab" ||
                event.key === "Escape"
              ) {
                // handled by the autocomplete component
                return false;
              }
            }

            if (ctx?.autocomplete()) {
              if (event.key === " " || event.key === "Enter") {
                ctx.setAutocomplete(null);
              } else {
                const state = view.state;
                const cursorPos = state.selection.from;

                const triggerChar = ctx.autocomplete()!.type === "mention"
                  ? "@"
                  : ctx.autocomplete()!.type === "channel"
                    ? "#"
                    : ctx.autocomplete()!.type === "command"
                      ? "/"
                      : ":";
                let mentionStart = -1;

                // search backward for trigger symbol
                for (let i = cursorPos - 1; i >= 0; i--) {
                  const char = state.doc.textBetween(i, i + 1);
                  if (char === triggerChar) {
                    mentionStart = i;
                    break;
                  }

                  // invalid characters for a mention query
                  if (char === " " || char === "\n" || char === "\t") {
                    ctx.setAutocomplete(null);
                    return false;
                  }
                }

                if (!ctx.autocomplete()) {
                  return false;
                }

                if (mentionStart === -1) {
                  ctx.setAutocomplete(null);
                  return false;
                }

                const currentQuery = state.doc.textBetween(
                  mentionStart + 1,
                  cursorPos,
                );

                let newQuery;
                if (event.key === "Backspace") {
                  if (cursorPos <= mentionStart + 1) {
                    ctx.setAutocomplete(null);
                    return false;
                  }
                  newQuery = currentQuery.slice(0, -1);
                } else if (
                  event.key.length === 1 && !event.ctrlKey && !event.metaKey &&
                  !event.altKey
                ) {
                  newQuery = currentQuery + event.key;
                } else {
                  return false;
                }

                ctx.setAutocomplete({
                  ...ctx.autocomplete()!,
                  query: newQuery,
                  ref: refElement() as any,
                });
              }
            }

            return false;
          },
          transformPastedHTML(html) {
            const markdown = turndown.turndown(html);
            const div = document.createElement("div");
            div.innerHTML = md.parser(md.lexer(markdown));
            return div.innerHTML;
          },
          editable: () => !(props.disabled ?? false),
          dispatchTransaction(tr) {
            const newState = view!.state.apply(tr);
            view!.updateState(newState);
            props.onChange?.(newState);
          },
        });

        view.focus();
      });

      onCleanup(() => {
        view?.destroy();
      });

      createEffect(() => {
        // update when placeholder changes too
        props.placeholder;

        view?.setProps({
          editable: () => !(props.disabled ?? false),
        });
      });

      return (
        <div
          class="editor"
          classList={{ "disabled": props.disabled ?? false }}
          tabindex={0}
          ref={editorRef!}
          role="textbox"
          aria-label="chat input"
          aria-placeholder={props.placeholder}
          aria-multiline="true"
        >
        </div>
      );
    },
  };
};
