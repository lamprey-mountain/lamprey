import { HighlightStyle } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

export const theme = EditorView.theme(
	{
		"&": {
			color: "oklch(var(--color-fg2))",
		},
		".cm-scroller": {
			fontFamily: "var(--font-mono)",
		},
		".cm-content": {
			caretColor: "#ff0000",
		},
		".cm-gutters": {
			backgroundColor: "oklch(var(--color-bg1))",
			color: "oklch(var(--color-fg4))",
			padding: "0 2px",
		},
		".cm-activeLine": {
			backgroundColor: "oklch(var(--color-bg1))",
		},
		// ".cm-selectionBackground, ::selection": {
		// 	// backgroundColor: "#3fa9c9",
		// 	backgroundColor: "#f00",
		// },
		".cm-selectionMatch": {
			backgroundColor: "oklch(var(--color-green) / 0.2)",
		},
	},
	{ dark: true },
);

export const highlight = HighlightStyle.define([
	{
		tag: [t.comment, t.quote],
		color: "oklch(var(--color-fg6))",
		fontStyle: "italic",
	},
	{
		tag: [t.keyword, t.modifier, t.inserted],
		color: "oklch(var(--color-magenta))",
	},
	{
		tag: [t.number, t.string, t.bool, t.regexp, t.literal],
		color: "oklch(var(--color-green))",
	},
	{
		tag: [t.heading, t.name, t.className, t.tagName],
		color: "oklch(var(--color-blue))",
	},
	{
		tag: [t.attributeName, t.propertyName, t.variableName, t.typeName],
		color: "oklch(var(--color-yellow))",
	},
	{ tag: [t.atom, t.meta, t.link], color: "oklch(var(--color-orange))" },
	{ tag: [t.deleted, t.standard(t.name)], color: "oklch(var(--color-red))" },
	{ tag: t.emphasis, fontStyle: "italic" },
	{ tag: t.strong, fontWeight: "bold" },
]);
