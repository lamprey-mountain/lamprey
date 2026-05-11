import type { CompletionContext } from "@codemirror/autocomplete";
import {
	Decoration,
	type DecorationSet,
	EditorView,
	MatchDecorator,
	ViewPlugin,
	type ViewUpdate,
	WidgetType,
} from "@codemirror/view";
import type { User } from "ts-sdk";

class MentionUserWidget extends WidgetType {
	constructor(readonly user: User) {
		super();
	}

	override eq(widget: MentionUserWidget): boolean {
		return this.user.id === widget.user.id;
	}

	override toDOM(): HTMLElement {
		return (<span class="user">@{this.user.name}</span>) as HTMLElement;
	}
}

// TODO: handle role mentions
// TODO: handle channel mentions
// TODO: handle media mentions
// TODO: make user mentions resolve room member nickname
// TODO: find out how to lazy load/search users

const users: User[] = [];

const mentionUserMatcher = new MatchDecorator({
	regexp: /@User\(([0-9]+)\)/g,
	decoration: (match) =>
		Decoration.replace({
			widget: new MentionUserWidget(
				users.find((i) => i.id.toString() === match[1]) ?? ({} as any),
			),
		}),
});

export function mentionsCompletions(context: CompletionContext) {
	const mention = context.matchBefore(/(?<=(\s|^)@)\w*/);

	if (mention) {
		return {
			from: mention.from,
			options: users.map((u) => ({
				label: u.name,
				type: "text",
				apply: `User(${u.id})`,
				detail: u.presence.status,
			})),
		};
	}

	// context.explicit;
	// let word = context.matchBefore(/\w*/);
	// if (word === null) return null;
	// // if (word.from === word.to && !context.explicit) return null;
	// return {
	//   from: word.from,
	//   options: [
	//     { label: "match", type: "keyword" },
	//     { label: "hello", type: "variable", info: "(World)" },
	//     { label: "magic", type: "text", apply: "⠁⭒*.✩.*⭒⠁", detail: "macro" },
	//   ],
	// };
}

const widgets = ViewPlugin.fromClass(
	class {
		placeholders: DecorationSet;
		constructor(view: EditorView) {
			this.placeholders = mentionUserMatcher.createDeco(view);
		}
		update(update: ViewUpdate) {
			this.placeholders = mentionUserMatcher.updateDeco(
				update,
				this.placeholders,
			);
		}
	},
	{
		decorations: (instance) => instance.placeholders,
		provide: (plugin) =>
			EditorView.atomicRanges.of((view) => {
				return view.plugin(plugin)?.placeholders || Decoration.none;
			}),
	},
);

// const view = new EditorView({
// 	doc: "Start document",
// 	parent: editorRef,
// 	extensions: [
// 		autocompletion({
// 			override: [myCompletions],
// 		}),
// 		placeholders,
// 	],
// });
