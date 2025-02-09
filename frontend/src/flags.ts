import { ReactiveSet } from "@solid-primitives/set";

type Env = {
	DEV: boolean;
};

type ImportMeta = {
	env: Env;
};

export const allFlags = [
	{
		id: "dev",
	},
	{
		id: "message_search",
	},
	{
		id: "room_member_list",
	},
	{
		id: "new_media",
	},
] as const;

type Flag = (typeof allFlags)[number]["id"];

const flagsDev: Flag[] = [
	"dev",
	"message_search",
	"room_member_list",
	"new_media",
];
const flagsProd: Flag[] = [];

export const flags = new ReactiveSet(
	(import.meta as unknown as ImportMeta).env.DEV ? flagsDev : flagsProd,
);
