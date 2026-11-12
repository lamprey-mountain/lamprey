import type { JSX } from "solid-js";

export function renderTemplate(
	template: string,
	...args: unknown[]
): JSX.Element {
	if (args.length === 0) return template;
	if (args.length === 1 && isPlainObject(args[0])) {
		const params = args[0];
		const parts = template.split(/\{\{\s*(\w+)\s*\}\}/g);
		const out: JSX.Element[] = [];
		for (let i = 0; i < parts.length; i++) {
			if (i % 2 === 0) {
				if (parts[i]) out.push(parts[i]);
			} else {
				const value = params[parts[i]];
				if (value !== undefined) out.push(value as JSX.Element);
			}
		}
		return out.length === 1 ? out[0] : out;
	}
	const parts = template.split(/\{\{\s*\w+\s*\}\}/g);
	const out: JSX.Element[] = [];
	for (let i = 0; i < parts.length; i++) {
		if (parts[i]) out.push(parts[i]);
		if (i < args.length) out.push(args[i] as JSX.Element);
	}
	return out.length === 1 ? out[0] : out;
}

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
	value !== null &&
	typeof value === "object" &&
	Object.getPrototypeOf(value) === Object.prototype;
