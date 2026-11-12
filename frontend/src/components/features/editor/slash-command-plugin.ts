import { type EditorState, Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import type { SlashCommands } from "@/lib/commands/registry";
import type { OptionDef } from "@/lib/commands/types";
import { buildCommandArgs, getEditorText, toDocPos } from "./command-args.ts";

export const slashCommandPluginKey = new PluginKey<DecorationSet>(
	"slash-command",
);

export function createSlashCommandPlugin(
	getCommands: () => SlashCommands | undefined,
	isValidUser?: (value: string) => boolean,
) {
	return new Plugin<DecorationSet>({
		key: slashCommandPluginKey,
		state: {
			init: (_, state) => calculate(getCommands(), isValidUser, state),
			apply(tr, prev, _oldState, newState) {
				if (!tr.docChanged) return prev.map(tr.mapping, tr.doc);
				return calculate(getCommands(), isValidUser, newState);
			},
		},
		props: {
			decorations(state) {
				return this.getState(state) ?? DecorationSet.empty;
			},
		},
	});
}

function calculate(
	commands: SlashCommands | undefined,
	isValidUser: ((value: string) => boolean) | undefined,
	state: EditorState,
): DecorationSet {
	const editorText = getEditorText(state);
	if (!editorText) return DecorationSet.empty;

	const text = editorText.text;
	if (!text.startsWith("/")) return DecorationSet.empty;

	const start = 1;
	const match = commands?.match(text);

	if (!match) {
		// still typing the name: mark it as a command, but not (yet) as invalid
		const space = text.search(/\s/);
		if (space === -1) {
			if (text.length <= 1) return DecorationSet.empty;
			return DecorationSet.create(state.doc, [
				Decoration.inline(start, toDocPos(editorText.segments, text.length), {
					class: "slash-command",
				}),
			]);
		}

		// unknown command with arguments: only style the command name
		if (space <= 1) return DecorationSet.empty;

		return DecorationSet.create(state.doc, [
			Decoration.inline(start, toDocPos(editorText.segments, space), {
				class: "slash-command invalid",
			}),
		]);
	}

	const args = buildCommandArgs(editorText, match.command, match.consumed);
	const decos: Decoration[] = [
		Decoration.inline(start, start + match.consumed, {
			class: "slash-command",
		}),
	];

	for (const slot of args.slots) {
		if (!slot.filled) continue;

		const isUser = slot.option.kind === "user";
		const hasValue = slot.valueEnd > slot.valueStart;
		const isMention = slot.mention;
		const valid =
			isMention ||
			!isValidUser ||
			(slot.value ? isValidUser(slot.value) : true);
		const inlineAt =
			isUser && hasValue && !isMention && slot.start === slot.valueStart;

		if (isUser && hasValue && !isMention) {
			decos.push(
				Decoration.inline(slot.valueStart, slot.valueEnd, {
					class: valid
						? "slash-arg-value user"
						: "slash-arg-value user invalid",
				}),
			);
			if (!inlineAt) {
				decos.push(
					Decoration.widget(slot.valueStart, () => createAtPrefix(!valid), {
						side: -1,
					}),
				);
			}
		}

		decos.push(
			Decoration.widget(
				slot.start,
				() => createArgPrefix(slot.option, slot.multiWord, inlineAt, !valid),
				{ side: -1 },
			),
		);

		if (slot.multiWord) {
			decos.push(
				Decoration.widget(args.textEnd, () => createQuote('"'), {
					side: 1,
				}),
			);
		}
	}

	// don't show argument labels until the user has moved past the command name
	const missing = args.slots.filter((slot) => !slot.filled);
	if (missing.length && text.length > match.consumed) {
		if (/\s$/.test(text)) {
			// fresh argument position: put the first label before the cursor so
			// typing starts right after `name:`
			const [first, ...rest] = missing.map((slot) => slot.option);

			decos.push(
				Decoration.widget(args.textEnd, () => createArgLabel(first, true), {
					side: -1,
				}),
			);

			decos.push(
				Decoration.widget(args.textEnd, () => createMissingTail(first, rest), {
					side: 1,
				}),
			);
		} else {
			// still editing a previous argument: keep the labels after the cursor
			decos.push(
				Decoration.widget(
					args.textEnd,
					() => createMissingArgs(missing.map((slot) => slot.option)),
					{ side: 1 },
				),
			);
		}
	}

	return DecorationSet.create(state.doc, decos);
}

function createAtPrefix(invalid = false): HTMLElement {
	const el = document.createElement("span");
	el.className = invalid ? "slash-arg-at invalid" : "slash-arg-at";
	el.textContent = "@";
	return el;
}

function createMissingTail(first: OptionDef, rest: OptionDef[]): HTMLElement {
	const wrap = document.createElement("span");
	wrap.className = "slash-arg-missing";

	const placeholder = document.createElement("span");
	placeholder.className = "slash-arg-placeholder";
	if (first.required) placeholder.classList.add("required");
	placeholder.textContent = argPlaceholder(first);
	wrap.append(placeholder);

	if (rest.length) wrap.append(createMissingArgs(rest));
	return wrap;
}

function argPlaceholder(option: OptionDef): string {
	switch (option.kind) {
		case "user":
			return "@username";
		case "duration":
			return "5m";
		default:
			return "text";
	}
}

function createArgPrefix(
	option: OptionDef,
	quote: boolean,
	at: boolean,
	invalid = false,
): HTMLElement {
	if (!quote && !at) return createArgLabel(option);

	const wrap = document.createElement("span");
	wrap.className = "slash-arg-prefix";
	wrap.append(createArgLabel(option));
	if (quote) wrap.append(createQuote('"'));
	if (at) wrap.append(createAtPrefix(invalid));
	return wrap;
}

function createQuote(quote: string): HTMLElement {
	const el = document.createElement("span");
	el.className = "slash-arg-quote";
	el.textContent = quote;
	return el;
}

function createArgLabel(option: OptionDef, current = false): HTMLElement {
	const el = document.createElement("span");
	el.className = "slash-arg-name";
	if (option.required) el.classList.add("required");
	if (current) el.classList.add("current");
	el.textContent = option.required ? option.name : `${option.name}?`;
	if (option.description) el.title = option.description;
	return el;
}

function createMissingArgs(options: OptionDef[]): HTMLElement {
	const wrap = document.createElement("span");
	wrap.className = "slash-arg-missing";

	for (const option of options) {
		const item = document.createElement("span");
		item.className = "slash-arg";
		if (option.required) item.classList.add("required");

		item.append(createArgLabel(option));

		const placeholder = document.createElement("span");
		placeholder.className = "slash-arg-placeholder";
		if (option.required) placeholder.classList.add("required");
		placeholder.textContent = argPlaceholder(option);
		item.append(placeholder);

		wrap.append(item);
	}

	return wrap;
}
