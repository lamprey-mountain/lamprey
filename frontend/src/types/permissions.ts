export type PermissionGroup =
	| "general"
	| "member"
	| "channel"
	| "message"
	| "voice"
	| "automod"
	| "reaction"
	| "thread";

export type PermissionCategory = "allow" | "deny" | "unset";
