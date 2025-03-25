import { TextDocument } from "sdk";
import { JSX } from "solid-js";

type Token = {
	text: string;
	type: "text" | "tag" | "start" | "end";
};

type SpanText = {
	type: "text";
	text: string;
};

type SpanTag = {
	type: "tag";
	name: string;
	args: Array<Text>;
};

type Text = Array<Span>;

type Span = SpanText | SpanTag;
type ParseSpan =
	| SpanText
	| {
		type: "tag";
		name: string;
		args: Array<Text>;
	}
	| { type: "endattr" }
	| { type: "eof" };

type ParseRes =
	| { type: "endattr"; spans: Array<Span> }
	| { type: "endinput"; spans: Array<Span> };

export function* tokenize(text: string): Generator<Token, void, unknown> {
	while (text.length) {
		const match = text.match(/[{}~]/);
		if (!match) {
			yield { text, type: "text" };
			break;
		}
		const chunk = text.slice(0, match.index!);
		if (chunk) yield { text: chunk, type: "text" };
		switch (match[0][0]) {
			case "~":
				yield { text: "~", type: "tag" };
				break;
			case "{":
				yield { text: "{", type: "start" };
				break;
			case "}":
				yield { text: "}", type: "end" };
				break;
		}
		text = text.slice(match[0].length + match.index!);
	}
}

export function parse(tokens: Generator<Token>) {
	const iter = {
		_peeked: null as null | Token,
		peek() {
			if (this._peeked) return this._peeked;
			const nval = tokens.next();
			this._peeked = nval.done ? null : nval.value;
			return this._peeked;
		},
		next() {
			if (this._peeked) {
				const it = this._peeked;
				this._peeked = null;
				return it;
			}
			const nval = tokens.next();
			return nval.done ? null : nval.value;
		},
	};

	const res = parseItem();
	if (res.type === "endattr") {
		while (true) {
			const t = iter.next();
			if (!t) break;
			res.spans.push({ type: "text", text: t.text });
		}
		return res.spans;
	} else if (res.type === "endinput") {
		return res.spans;
	} else {
		throw new Error("unreachable");
	}

	function parseItem(): ParseRes {
		const spans: Array<Span> = [];
		while (true) {
			const span = parseText();
			if (span.type === "text") {
				spans.push({ type: "text", text: span.text });
			} else if (span.type === "eof") {
				break;
			} else if (span.type === "endattr") {
				return { type: "endattr", spans };
			} else if (span.type === "tag") {
				spans.push({ type: "tag", name: span.name, args: span.args });
			}
		}
		return { type: "endinput", spans };
	}

	function parseAttr(): Text | null {
		const tok = iter.peek();
		if (!tok) return null;
		if (tok.type !== "start") return null;
		iter.next();
		const t = parseItem();
		if (t.type === "endattr") {
			return t.spans;
		} else if (t.type === "endinput") return t.spans;
		else throw new Error("unreachable");
	}

	function parseText(): ParseSpan {
		const tok = iter.next();
		if (!tok) return { type: "eof" };

		if (tok.type === "start") {
			return { type: "text", text: "{" };
		} else if (tok.type === "end") {
			return { type: "endattr" };
		} else if (tok.type === "text") {
			return { type: "text", text: tok.text };
		} else if (tok.type === "tag") {
			const n = iter.next();
			if (!n) return { type: "eof" };
			if (n.type === "text") {
				const name = n.text;
				const args = [];
				while (true) {
					const arg = parseAttr();
					if (arg) {
						args.push(arg);
					} else {
						break;
					}
				}
				return { type: "tag", args, name };
			} else if (n.type === "start") {
				return { type: "text", text: "{" };
			} else if (n.type === "end") {
				return { type: "text", text: "}" };
			} else if (n.type === "tag") {
				return { type: "text", text: "~" };
			}
		}
		throw new Error("unreachable");
	}
}

function stringifySpan(s: Span): string {
	if (s.type === "text") {
		return s.text;
	} else {
		return s.args.map((i) => stringify(i)).join(" ");
	}
}

function stringify(s: Text): string {
	return s.map((i) => stringifySpan(i)).join(" ");
}

export function transformInline(source: string): JSX.Element {
	const parsed = parse(tokenize(source));
	return parsed.map((i) => toElement(i));

	function toElement(item: Text | Span): JSX.Element {
		if (Array.isArray(item)) {
			return item.map((i) => toElement(i));
		}
		if (item.type === "text") {
			return item.text;
		}
		switch (item.name) {
			case "b":
				return <b>{toElement(item.args[0])}</b>;
			case "em":
				return <em>{toElement(item.args[0])}</em>;
			case "s":
				return <s>{toElement(item.args[0])}</s>;
			case "a":
				return <a href={stringify(item.args[0])}>{toElement(item.args[1])}</a>;
			case "code": {
				if (item.args[1]) {
					return (
						<code class={`lang-${stringify(item.args[1])}`}>
							{toElement(item.args[0])}
						</code>
					);
				} else {
					return <code>{toElement(item.args[0])}</code>;
				}
			}
			// case "mention-": return <span>{toElement(item.args[0])}</span>;
			default:
				return <span class="error">unknown tag {item.name}</span>;
		}
	}
}

export function transformBlock(doc: TextDocument): JSX.Element {
	if (typeof doc === "string") {
		return transformInline(doc);
	} else if (Array.isArray(doc)) {
		return doc.map((i) => transformBlock(i));
	} else {
		switch (doc.type) {
			case "Paragraph":
				return <p>{transformBlock(doc.text)}</p>;
			case "Heading": {
				switch (doc.level) {
					case 1:
						return <h1>{transformBlock(doc.text)}</h1>;
					case 2:
						return <h2>{transformBlock(doc.text)}</h2>;
					case 3:
						return <h3>{transformBlock(doc.text)}</h3>;
					case 4:
						return <h4>{transformBlock(doc.text)}</h4>;
					case 5:
						return <h5>{transformBlock(doc.text)}</h5>;
					case 6:
						return <h6>{transformBlock(doc.text)}</h6>;
					default:
						return <div class="error">Unknown header level: {doc.level}</div>;
				}
			}
			case "Blockquote":
				return <blockquote>{transformBlock(doc.text)}</blockquote>;
			case "Code": {
				if (doc.lang) {
					return (
						<pre><code class={`lang-${doc.lang}`}>{transformBlock(doc.text)}</code></pre>
					);
				} else {
					return <pre>{transformBlock(doc.text)}</pre>;
				}
			}
			case "ListUnordered":
				return <ul>{doc.items.map((i) => <li>{transformBlock(i)}</li>)}</ul>;
			case "ListOrdered":
				return <ol>{doc.items.map((i) => <li>{transformBlock(i)}</li>)}</ol>;
			default:
				return <div class="error">Unknown block type: {doc.type}</div>;
		}
	}
}
