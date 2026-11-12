import type { ReferenceElement } from "@floating-ui/dom";
import { Plugin, PluginKey, TextSelection } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import type {
	AutocompleteContext,
	AutocompleteItem,
	AutocompleteKind,
} from "@/contexts/autocomplete";
import type { SlashCommands } from "@/lib/commands/registry";
import { quoteArg } from "@/lib/commands/tokenize";
import { getActiveSlot, getCommandArgs } from "./command-args.ts";

export const autocompleteKey = new PluginKey("autocomplete");

const LINE_HEIGHT = 18;

function getTriggerChar(type: AutocompleteKind["type"]): string {
	switch (type) {
		case "mention":
			return "@";
		case "channel":
			return "#";
		case "emoji":
			return ":";
		case "command":
			return "/";
		case "commandArg":
			return "";
	}
}

function findTriggerPosition(
	view: EditorView,
	triggerChar: string,
): number | null {
	const state = view.state;
	const cursorPos = state.selection.from;

	// Search backward from cursor to find the trigger character
	for (let i = cursorPos - 1; i >= 0; i--) {
		const char = state.doc.textBetween(i, i + 1);
		if (char === triggerChar) {
			return i;
		}
		// Stop at whitespace - trigger must be on current "word"
		if (char === " " || char === "\n" || char === "\t") {
			return null;
		}
	}
	return null;
}

function createRefElement(view: EditorView): ReferenceElement {
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
}

function getCurrentQuery(view: EditorView, triggerPos: number): string {
	const state = view.state;
	const cursorPos = state.selection.from;
	return state.doc.textBetween(triggerPos + 1, cursorPos);
}

function isValidTriggerChar(char: string): char is "@" | "#" | ":" | "/" {
	return char === "@" || char === "#" || char === ":" || char === "/";
}

function getAutocompleteType(char: string): AutocompleteKind["type"] | null {
	switch (char) {
		case "@":
			return "mention"; // Will be refined based on query
		case "#":
			return "channel";
		case ":":
			return "emoji";
		case "/":
			return "command";
		default:
			return null;
	}
}

export function createAutocompletePlugin(
	autocomplete: AutocompleteContext,
	channelId: () => string,
	roomId: () => string,
	slashCommands?: () => SlashCommands | undefined,
): Plugin {
	const handleTrigger = (view: EditorView, triggerChar: string): boolean => {
		const state = view.state;
		const type = getAutocompleteType(triggerChar);

		if (!type) return false;

		// Only trigger at start of block for commands
		// The "/" should be right at the start of the parent block
		if (type === "command") {
			const $pos = state.selection.$from;
			const parentStart = $pos.start($pos.depth);
			// Trigger char is at cursorPos - 1, check if that equals parentStart
			if (state.selection.from - 1 !== parentStart) {
				return false;
			}
		}

		// Find the trigger position to get the initial query
		const triggerPos = findTriggerPosition(view, triggerChar);
		const initialQuery =
			triggerPos !== null ? getCurrentQuery(view, triggerPos) : "";

		const refElement = createRefElement(view);

		// For "@" trigger, show combined mention/role/everyone autocomplete
		if (type === "mention") {
			autocomplete.show(refElement, {
				type: "mention",
				onSelect: (item) => {
					applyAutocompleteReplacement(view, triggerChar, "mention", item);
				},
				channelId: channelId(),
				roomId: roomId(),
			});
		} else if (type === "channel") {
			autocomplete.show(refElement, {
				type: "channel",
				onSelect: (item) => {
					applyAutocompleteReplacement(view, triggerChar, "channel", item);
				},
				channelId: channelId(),
			});
		} else if (type === "emoji") {
			autocomplete.show(refElement, {
				type: "emoji",
				onSelect: (id, name, char) => {
					applyAutocompleteReplacement(view, triggerChar, "emoji", {
						type: "emoji" as const,
						id,
						name,
						char,
					});
				},
				channelId: channelId(),
			});
		} else if (type === "command") {
			autocomplete.show(refElement, {
				type: "command",
				onSelect: (command) => {
					applyAutocompleteReplacement(view, triggerChar, "command", {
						type: "command" as const,
						command,
						id: command,
					});
				},
				channelId: channelId(),
			});
		}

		// Set initial query immediately after show
		autocomplete.updateQuery(initialQuery);

		return true;
	};

	const applyAutocompleteReplacement = (
		view: EditorView,
		triggerChar: string,
		type: AutocompleteKind["type"],
		item: AutocompleteItem,
	) => {
		const state = view.state;
		const triggerPos = findTriggerPosition(view, triggerChar);
		if (triggerPos === null) {
			return;
		}

		const to = state.selection.to;

		let node;
		if (type === "emoji") {
			const emojiItem = item as Extract<AutocompleteItem, { type: "emoji" }>;
			if (emojiItem.char) {
				// Unicode emoji - insert as emojiUnicode atom node
				node = state.schema.nodes.emojiUnicode.create({
					char: emojiItem.char,
				});
			} else {
				// Custom emoji
				node = state.schema.nodes.emojiCustom.create({
					id: emojiItem.id,
					name: emojiItem.name,
				});
			}
		} else if (type === "command") {
			const commandItem = item as Extract<
				AutocompleteItem,
				{ type: "command" }
			>;
			// For commands, replace from start of the current block/line
			const $pos = state.selection.$from;
			const parentStart = $pos.start($pos.depth);
			const commandText = `/${commandItem.id} `;
			let tr = state.tr.replaceWith(
				parentStart,
				to,
				state.schema.text(commandText),
			);
			// Cursor should be after the command text
			tr = tr.setSelection(
				TextSelection.create(tr.doc, parentStart + commandText.length),
			);
			autocomplete.hide();
			view.dispatch(tr);
			return;
		} else if (type === "mention") {
			// Handle user, role, or everyone mention
			if (item.type === "user") {
				node = state.schema.nodes.mention.create({
					user: item.user_id,
					name: item.name ?? "",
				});
			} else if (item.type === "role") {
				node = state.schema.nodes.mentionRole.create({
					role: item.role_id,
					name: item.name ?? "",
				});
			} else if (item.type === "everyone") {
				// @everyone - insert as atomic node
				node = state.schema.nodes.mentionEveryone.create();
			}
		} else {
			// channel mention
			const channelItem = item as Extract<
				AutocompleteItem,
				{ type: "channel" }
			>;
			const nodeType = state.schema.nodes.mentionChannel;
			const attrs = { channel: channelItem.channel_id, name: channelItem.name };
			node = nodeType.create(attrs);
		}

		if (!node) return;
		let tr = state.tr.replaceWith(triggerPos, to, node);
		const posAfter = tr.mapping.map(to);
		tr = tr.insert(posAfter, state.schema.text(" ", []));
		tr = tr.setSelection(TextSelection.create(tr.doc, posAfter + 1));

		view.dispatch(tr);
		autocomplete.hide();
	};

	const getArgs = (view: EditorView) =>
		getCommandArgs(view.state, slashCommands?.());

	const isArgTrigger = (query: string) => /^[@#:]/.test(query);

	const syncCommandArg = (view: EditorView): boolean => {
		const kind = autocomplete.state.kind;
		if (kind?.type !== "commandArg") return false;

		const args = getArgs(view);
		if (!args) return false;

		const slot = getActiveSlot(args, view.state.selection.from);
		if (!slot || slot.index !== kind.index) return false;
		if (isArgTrigger(slot.query)) return false;

		autocomplete.updateQuery(slot.query);
		autocomplete.setReference(createRefElement(view));
		return true;
	};

	const showCommandArg = (view: EditorView): boolean => {
		const args = getArgs(view);
		if (!args) return false;

		const slot = getActiveSlot(args, view.state.selection.from);
		if (!slot) return false;
		if (slot.option.kind !== "user" && slot.option.kind !== "duration") {
			return false;
		}
		if (isArgTrigger(slot.query)) return false;

		autocomplete.show(createRefElement(view), {
			type: "commandArg",
			option: slot.option,
			index: slot.index,
			onSelect: (value, item) => applyCommandArg(view, slot.index, value, item),
			channelId: channelId(),
			roomId: roomId(),
		});
		autocomplete.updateQuery(slot.query);
		return true;
	};

	const applyCommandArg = (
		view: EditorView,
		index: number,
		value: string,
		item?: AutocompleteItem,
	) => {
		const args = getArgs(view);
		const slot = args?.slots[index];
		if (!args || !slot) {
			autocomplete.hide();
			return;
		}

		const state = view.state;
		const replacement =
			slot.option.kind === "user" && item?.type === "user"
				? state.schema.nodes.mention.create({
						user: item.user_id,
						name: item.name,
					})
				: state.schema.text(quoteArg(value));

		const to = slot.filled ? slot.end : slot.start;
		let tr = state.tr.replaceWith(slot.start, to, replacement);

		let pos = slot.start + replacement.nodeSize;
		const before = pos > 1 ? tr.doc.textBetween(pos - 1, pos) : "";
		if (before !== " ") {
			tr = tr.insert(pos, state.schema.text(" "));
			pos++;
		}
		tr = tr.setSelection(TextSelection.create(tr.doc, pos));

		autocomplete.hide();
		view.dispatch(tr);
	};

	const updateQuery = (view: EditorView) => {
		const currentKind = autocomplete.state.kind;
		if (!currentKind) return;

		const triggerChar = getTriggerChar(currentKind.type);
		const triggerPos = findTriggerPosition(view, triggerChar);

		if (triggerPos === null) {
			autocomplete.hide();
			return;
		}

		const query = getCurrentQuery(view, triggerPos);
		if (query !== autocomplete.state.query) {
			autocomplete.updateQuery(query);
		}

		// Keep reference element updated
		const refElement = createRefElement(view);
		autocomplete.setReference(refElement);
	};

	const handleKeyDown = (_view: EditorView, event: KeyboardEvent): boolean => {
		const currentKind = autocomplete.state.kind;

		if (!currentKind) {
			// Check for trigger characters
			if (isValidTriggerChar(event.key) && event.key.length === 1) {
				// Let the update handler process the trigger
				return false;
			}
			return false;
		}

		// Autocomplete is active - handle navigation
		if (event.key === "ArrowUp") {
			event.preventDefault();
			autocomplete.navigate("up");
			return true;
		}

		if (event.key === "ArrowDown") {
			event.preventDefault();
			autocomplete.navigate("down");
			return true;
		}

		if (event.key === "Enter" || event.key === "Tab") {
			event.preventDefault();
			autocomplete.select();
			return true;
		}

		if (event.key === "Escape") {
			event.preventDefault();
			autocomplete.hide();
			return true;
		}

		if (event.key === " ") {
			// Close on space
			autocomplete.hide();
			return false;
		}

		return false;
	};

	return new Plugin({
		key: autocompleteKey,
		view(_editorView) {
			return {
				update(view, prevState) {
					const currentKind = autocomplete.state.kind;
					const docChanged = !prevState.doc.eq(view.state.doc);

					if (currentKind?.type === "commandArg") {
						// Argument autocomplete is active - keep it in sync
						if (syncCommandArg(view)) return;
						autocomplete.hide();
					} else if (currentKind) {
						// Autocomplete is active - update query and position
						updateQuery(view);
						if (autocomplete.state.kind) return;
						// trigger disappeared (e.g. space after a command name)
						if (docChanged && showCommandArg(view)) return;
						return;
					}

					if (!docChanged) return;

					// Inside a command's arguments? Show users/durations for the slot
					if (showCommandArg(view)) return;

					const cursorPos = view.state.selection.from;
					if (cursorPos <= 0) return;

					const charBefore = view.state.doc.textBetween(
						cursorPos - 1,
						cursorPos,
					);

					if (!isValidTriggerChar(charBefore)) return;

					// Check what's before the trigger character
					const $pos = view.state.selection.$from;
					const parentStart = $pos.start($pos.depth);

					// If we're at the start of the parent block, that's valid
					if (cursorPos - 1 === parentStart) {
						handleTrigger(view, charBefore);
						return;
					}

					// Otherwise, check if preceded by whitespace or start of document
					// cursorPos - 2 could be negative if trigger is at position 1
					const charBeforeTrigger =
						cursorPos >= 2
							? view.state.doc.textBetween(cursorPos - 2, cursorPos - 1)
							: "";

					if (
						charBeforeTrigger === "" || // Start of document or position 1
						charBeforeTrigger === " " ||
						charBeforeTrigger === "\n" ||
						charBeforeTrigger === "\t"
					) {
						handleTrigger(view, charBefore);
					}
				},
			};
		},
		props: {
			handleKeyDown,
		},
	});
}
