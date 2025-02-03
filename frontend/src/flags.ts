import { ReactiveSet } from "@solid-primitives/set";

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

const flagsDev: Flag[] = ["message_search", "room_member_list"];
const flagsProd: Flag[] = [];

export const flags = new ReactiveSet(
	(import.meta as unknown as ImportMeta).env.DEV ? flagsDev : flagsProd,
);
