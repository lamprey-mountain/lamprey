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
		id: "thread_member_list",
	},
	{
		id: "service_worker_media",
	},
	{
		id: "inbox",
	},
	{
		id: "friends",
	},
	{
		id: "two_tier_nav",
	},
	{
		id: "nav_header",
	},
] as const;

type Flag = (typeof allFlags)[number]["id"];

const flagsDev: Flag[] = [
	"dev",
	"message_search",
	"room_member_list",
	"thread_member_list",
	"service_worker_media",
	"inbox",
	"friends",
	"two_tier_nav",
	"nav_header",
];

const flagsProd: Flag[] = [
	"service_worker_media",
	"room_member_list",
	"thread_member_list",
	"two_tier_nav",
];

export const flags = new ReactiveSet(
	import.meta.env.DEV ? flagsDev : flagsProd,
);
