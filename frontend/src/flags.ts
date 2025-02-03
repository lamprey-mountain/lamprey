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
	{
		id: "room_member_list",
	},
] as const;

type Flag = (typeof allFlags)[number]["id"];

const flagsDev: Set<Flag> = new Set(["message_search", "room_member_list"]);
const flagsProd: Set<Flag> = new Set([]);

export const flags = (import.meta as unknown as ImportMeta).env.DEV
	? flagsDev
	: flagsProd;
