type Env = {
	DEV: boolean;
};

type ImportMeta = {
	env: Env;
};

export const allFlags = [
	{
		id: "message_search",
	},
] as const;

type Flag = (typeof allFlags)[number]["id"];

const flagsDev: Set<Flag> = new Set(["message_search"]);
const flagsProd: Set<Flag> = new Set([]);

export const flags = (import.meta as unknown as ImportMeta).env.DEV
	? flagsDev
	: flagsProd;
