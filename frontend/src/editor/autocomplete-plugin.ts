import { Plugin, PluginKey, TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { ReferenceElement } from "@floating-ui/dom";
import { useAutocomplete } from "../contexts/autocomplete";
import type { AutocompleteKind } from "../contexts/autocomplete";

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
	channelId: () => string,
	roomId: () => string,
): Plugin {
	const autocomplete = useAutocomplete();

	const handleTrigger = (
		view: EditorView,
		triggerChar: string,
	): boolean => {
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
		const initialQuery = triggerPos !== null
			? getCurrentQuery(view, triggerPos)
			: "";

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
		} else {
			autocomplete.show(refElement, {
				type,
				onSelect: (item: any) => {
					applyAutocompleteReplacement(view, triggerChar, type, item);
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
		item: any,
	) => {
		const state = view.state;
		const triggerPos = findTriggerPosition(view, triggerChar);
		if (triggerPos === null) {
			return;
		}

		const to = state.selection.to;

		let node;
		if (type === "emoji") {
			if (item.char) {
				node = state.schema.text(item.char);
			} else {
				node = state.schema.nodes.emoji.create({
					id: item.id,
					name: item.label ?? item.name,
				});
			}
		} else if (type === "command") {
			// For commands, replace from start of the current block/line
			const $pos = state.selection.$from;
			const parentStart = $pos.start($pos.depth);
			const commandText = `/${item.id}`;
			let tr = state.tr.replaceWith(
				parentStart,
				to,
				state.schema.text(commandText),
			);
			// Cursor should be after the command text
			tr = tr.setSelection(
				TextSelection.create(tr.doc, parentStart + commandText.length),
			);
			view.dispatch(tr);
			autocomplete.hide();
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
				// @everyone - insert as text, cursor after
				let tr = state.tr.replaceWith(
					triggerPos,
					to,
					state.schema.text("@everyone"),
				);
				const posAfter = tr.mapping.map(to);
				tr = tr.setSelection(TextSelection.create(tr.doc, posAfter));
				view.dispatch(tr);
				autocomplete.hide();
				return;
			}
		} else {
			// channel mention
			const nodeType = state.schema.nodes.mentionChannel;
			const attrs = { channel: item.id, name: item.name ?? "" };
			node = nodeType.create(attrs);
		}

		let tr = state.tr.replaceWith(triggerPos, to, node);
		const posAfter = tr.mapping.map(to);
		tr = tr.insert(posAfter, state.schema.text(" ", []));
		tr = tr.setSelection(TextSelection.create(tr.doc, posAfter + 1));

		view.dispatch(tr);
		autocomplete.hide();
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

	const handleKeyDown = (view: EditorView, event: KeyboardEvent): boolean => {
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

					if (currentKind) {
						// Autocomplete is active - update query and position
						updateQuery(view);
						return;
					}

					// Check for new trigger (only if doc changed and we're not already showing)
					if (!docChanged) return;

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
					const charBeforeTrigger = cursorPos >= 2
						? view.state.doc.textBetween(cursorPos - 2, cursorPos - 1)
						: "";

					if (
						charBeforeTrigger === "" || // Start of document or position 1
						charBeforeTrigger === " " || charBeforeTrigger === "\n" ||
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
