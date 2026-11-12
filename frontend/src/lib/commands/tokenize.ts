export type CommandToken = {
	/** raw source text, including quotes */
	raw: string;
	/** unescaped value without surrounding quotes */
	value: string;
	start: number;
	end: number;
	/** offsets of the value, excluding surrounding quotes */
	valueStart: number;
	valueEnd: number;
	quoted: boolean;
};

const QUOTES = new Set(['"', "'"]);

/** split command arguments into tokens, supporting ' and " quotes and escapes */
export function tokenizeArgs(input: string): CommandToken[] {
	const tokens: CommandToken[] = [];
	let i = 0;

	while (i < input.length) {
		while (i < input.length && isWhitespace(input[i])) i++;
		if (i >= input.length) break;

		const start = i;
		let value = "";
		let quoted = false;

		if (QUOTES.has(input[i])) {
			quoted = true;
			const quote = input[i];
			const valueStart = ++i;
			let closed = false;

			while (i < input.length) {
				const char = input[i];
				if (char === "\\") {
					const [escaped, next] = readEscape(input, i);
					value += escaped;
					i = next;
					continue;
				}
				if (char === quote) {
					closed = true;
					i++;
					break;
				}
				value += char;
				i++;
			}

			tokens.push({
				raw: input.slice(start, i),
				value,
				start,
				end: i,
				valueStart,
				valueEnd: closed ? i - 1 : i,
				quoted,
			});
			continue;
		}

		const valueStart = i;
		while (i < input.length && !isWhitespace(input[i])) {
			const char = input[i];
			if (char === "\\") {
				const [escaped, next] = readEscape(input, i);
				value += escaped;
				i = next;
				continue;
			}
			value += char;
			i++;
		}

		tokens.push({
			raw: input.slice(start, i),
			value,
			start,
			end: i,
			valueStart,
			valueEnd: i,
			quoted,
		});
	}

	return tokens;
}

/** resolve escape sequences in a (possibly incomplete) argument fragment */
export function unescapeArg(input: string): string {
	let result = "";
	let i = 0;

	while (i < input.length) {
		if (input[i] === "\\") {
			const [escaped, next] = readEscape(input, i);
			result += escaped;
			i = next;
			continue;
		}
		result += input[i];
		i++;
	}

	return result;
}

/** quote and escape an argument value so it stays a single token */
export function quoteArg(value: string): string {
	if (!/[\s"'\\\n\t]/.test(value)) return value;
	return `"${value
		.replace(/\\/g, "\\\\")
		.replace(/"/g, '\\"')
		.replace(/\n/g, "\\n")
		.replace(/\t/g, "\\t")}"`;
}

function isWhitespace(char: string): boolean {
	return char === " " || char === "\n" || char === "\t";
}

function readEscape(input: string, i: number): [string, number] {
	const next = input[i + 1];
	if (next === undefined) return ["\\", i + 1];

	switch (next) {
		case "n":
			return ["\n", i + 2];
		case "t":
			return ["\t", i + 2];
		case "r":
			return ["\r", i + 2];
		case "\\":
			return ["\\", i + 2];
		case '"':
			return ['"', i + 2];
		case "'":
			return ["'", i + 2];
		default:
			return [next, i + 2];
	}
}
