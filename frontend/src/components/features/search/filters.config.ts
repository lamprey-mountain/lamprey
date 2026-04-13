import type { Node } from "prosemirror-model";
import type { User } from "sdk";
import type {
	useChannels,
	useRoles,
	useRoomMembers,
	useThreadMembers,
	useUsers,
} from "@/api";
import type { RoomT, ThreadT } from "@/types";
import { schema } from "./schema";

// ---------------------------------------------------------------------------
// Filter registry – single source of truth for all search filters
// ---------------------------------------------------------------------------

export type SearchFilterValueType = "id" | "value" | "date";

export interface SearchContext {
	users: ReturnType<typeof useUsers>;
	channels: ReturnType<typeof useChannels>;
	roomMembers: ReturnType<typeof useRoomMembers>;
	threadMembers: ReturnType<typeof useThreadMembers>;
	roles: ReturnType<typeof useRoles>;
	roomThreads: () => ThreadT[];
	roomId: string | null;
	channel?: ThreadT;
	room?: RoomT;
}

export interface SuggestionItem {
	id: string;
	label: string;
	user?: User;
	channel?: ThreadT;
	rawValue?: string;
	onSelect: () => void;
}

/** Internal AST node representation for a filter */
export interface FilterASTNode {
	type: string;
	value: string;
	name?: string;
	negated: boolean;
}

export interface SearchFilterDef {
	/** The filter keyword (e.g. "author", "channel") */
	name: string;
	/** What kind of value the filter expects */
	valueType: SearchFilterValueType;
	/** Whether the node stores a human-readable name in addition to the value */
	hasNameAttr: boolean;
	/** Produce suggestion items given the current partial query */
	getSuggestions: (query: string, ctx: SearchContext) => SuggestionItem[];
	resolveDisplayData?: (
		value: string,
		ctx: SearchContext,
	) => { name?: string; user?: User; channel?: ThreadT };
	/** Convert a ProseMirror node → our intermediate FilterASTNode */
	toAST: (node: Node) => FilterASTNode;
	/** Convert a FilterASTNode → the backend query string fragment */
	toBackendQuery: (node: FilterASTNode) => string[];
	/** Convert a FilterASTNode → a ProseMirror node */
	toPMNode: (ast: FilterASTNode) => Node;
}

// ---------------------------------------------------------------------------
// Helper: extract member IDs from caches
// ---------------------------------------------------------------------------

function getThreadMemberIds(threadId: string, ctx: SearchContext): string[] {
	return [...ctx.threadMembers.cache.entries()]
		.filter(([key]) => key.startsWith(`${threadId}:`))
		.map(([, member]) => member.user_id);
}

function getRoomMemberIds(roomId: string, ctx: SearchContext): string[] {
	return [...ctx.roomMembers.cache.entries()]
		.filter(([key]) => key.startsWith(`${roomId}:`))
		.map(([, member]) => member.user_id);
}

// ---------------------------------------------------------------------------
// Individual filter definitions
// ---------------------------------------------------------------------------

export const authorFilter: SearchFilterDef = {
	name: "author",
	valueType: "id",
	hasNameAttr: true,
	getSuggestions(query, ctx) {
		const allIds = [
			...new Set([
				...(ctx.channel ? getThreadMemberIds(ctx.channel.id, ctx) : []),
				...(ctx.roomId ? getRoomMemberIds(ctx.roomId, ctx) : []),
			]),
		];
		const q = query.toLowerCase();
		const filtered = allIds
			.map((id) => ctx.users.cache.get(id))
			.filter((u): u is NonNullable<typeof u> => Boolean(u))
			.filter(
				(u) =>
					u.name.toLowerCase().includes(q) || u.id.toLowerCase().includes(q),
			);

		return filtered.slice(0, 10).map((u) => ({
			id: `author-${u.id}`,
			label: u.name,
			user: u,
			onSelect: () => {},
		}));
	},
	resolveDisplayData(value, ctx) {
		const user = ctx.users.cache.get(value);
		return {
			name: user?.name ?? value,
			user,
		};
	},
	toAST(node) {
		return {
			type: "author",
			value: node.attrs.id as string,
			name: node.attrs.name as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery(ast) {
		const prefix = ast.negated ? "-" : "+";
		return [`${prefix}author_id:${ast.value}`];
	},
	toPMNode(ast) {
		return schema.nodes.author.create({
			id: ast.value,
			name: ast.name ?? ast.value,
			negated: ast.negated,
		});
	},
};

export const channelFilter: SearchFilterDef = {
	name: "channel",
	valueType: "id",
	hasNameAttr: true,
	getSuggestions(query, ctx) {
		const threads = ctx.roomThreads();
		const q = query.toLowerCase();
		const filtered = q
			? threads.filter(
					(t) =>
						t.name.toLowerCase().includes(q) || t.id.toLowerCase().includes(q),
				)
			: threads;

		return filtered.slice(0, 10).map((t) => ({
			id: `channel-${t.id}`,
			label: t.name,
			channel: t,
			onSelect: () => {},
		}));
	},
	resolveDisplayData(value, ctx) {
		const channel = ctx.roomThreads().find((t) => t.id === value);
		return {
			name: channel?.name ?? value,
			channel,
		};
	},
	toAST(node) {
		return {
			type: "channel",
			value: node.attrs.id as string,
			name: node.attrs.name as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery(ast) {
		const prefix = ast.negated ? "-" : "+";
		return [`${prefix}channel_id:${ast.value}`];
	},
	toPMNode(ast) {
		return schema.nodes.channel.create({
			id: ast.value,
			name: ast.name ?? ast.value,
			negated: ast.negated,
		});
	},
};

export const beforeFilter: SearchFilterDef = {
	name: "before",
	valueType: "date",
	hasNameAttr: false,
	getSuggestions: () => [],
	toAST(node) {
		return {
			type: "before",
			value: node.attrs.date as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery: () => [], // handled specially by the compiler
	toPMNode(ast) {
		return schema.nodes.before.create({
			date: ast.value,
			negated: ast.negated,
		});
	},
};

export const afterFilter: SearchFilterDef = {
	name: "after",
	valueType: "date",
	hasNameAttr: false,
	getSuggestions: () => [],
	toAST(node) {
		return {
			type: "after",
			value: node.attrs.date as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery: () => [], // handled specially by the compiler
	toPMNode(ast) {
		return schema.nodes.after.create({ date: ast.value, negated: ast.negated });
	},
};

const HAS_VALUE_MAP: Record<string, string> = {
	attachment: "metadata_fast.has_attachment:true",
	image: "metadata_fast.has_image:true",
	audio: "metadata_fast.has_audio:true",
	video: "metadata_fast.has_video:true",
	link: "metadata_fast.has_link:true",
	embed: "metadata_fast.has_embed:true",
};

export const hasFilter: SearchFilterDef = {
	name: "has",
	valueType: "value",
	hasNameAttr: false,
	getSuggestions(query) {
		const options = Object.keys(HAS_VALUE_MAP);
		const q = query.toLowerCase();
		const filtered = q ? options.filter((o) => o.includes(q)) : options;
		return filtered.map((v) => ({
			id: `has-${v}`,
			label: v,
			onSelect: () => {},
		}));
	},
	toAST(node) {
		return {
			type: "has",
			value: node.attrs.value as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery(ast) {
		const backendVal = HAS_VALUE_MAP[ast.value];
		if (!backendVal) return [];
		const prefix = ast.negated ? "-" : "+";
		return [`${prefix}${backendVal}`];
	},
	toPMNode(ast) {
		return schema.nodes.has.create({ value: ast.value, negated: ast.negated });
	},
};

export const pinnedFilter: SearchFilterDef = {
	name: "pinned",
	valueType: "value",
	hasNameAttr: false,
	getSuggestions(query) {
		const options = ["true", "false"];
		const q = query.toLowerCase();
		const filtered = q ? options.filter((o) => o.includes(q)) : options;
		return filtered.map((v) => ({
			id: `pinned-${v}`,
			label: v,
			onSelect: () => {},
		}));
	},
	toAST(node) {
		return {
			type: "pinned",
			value: node.attrs.value as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery(ast) {
		return [`+metadata_fast.pinned:${ast.value}`];
	},
	toPMNode(ast) {
		return schema.nodes.pinned.create({
			value: ast.value,
			negated: ast.negated,
		});
	},
};

type Mentionable = {
	id: string;
	name: string;
	type: "user" | "role" | "special";
	user?: User;
};

export const mentionsFilter: SearchFilterDef = {
	name: "mentions",
	valueType: "id",
	hasNameAttr: true,
	getSuggestions(query, ctx) {
		const allIds = [
			...new Set([
				...(ctx.channel ? getThreadMemberIds(ctx.channel.id, ctx) : []),
				...(ctx.roomId ? getRoomMemberIds(ctx.roomId, ctx) : []),
			]),
		];

		const users = allIds
			.map((id) => ctx.users.cache.get(id))
			.filter((u): u is NonNullable<typeof u> => Boolean(u))
			.filter(
				(u) =>
					u.name.toLowerCase().includes(query.toLowerCase()) ||
					u.id.toLowerCase().includes(query.toLowerCase()),
			)
			.map(
				(u) =>
					({
						id: `user-${u.id}`,
						name: u.name,
						type: "user" as const,
						user: u,
					}) satisfies Mentionable,
			);

		const roomRoles = ctx.roomId
			? [...ctx.roles.cache.values()].filter(
					(r) =>
						r.room_id === ctx.roomId &&
						r.name.toLowerCase().includes(query.toLowerCase()),
				)
			: [];
		const roleSuggestions: Mentionable[] = roomRoles.map(
			(r) =>
				({
					id: `role-${r.id}`,
					name: r.name,
					type: "role" as const,
				}) satisfies Mentionable,
		);

		const specialCandidates: Mentionable[] = [
			{ id: "everyone-room", name: "@room", type: "special" },
			{ id: "everyone-thread", name: "@thread", type: "special" },
		];
		const special = specialCandidates.filter((s) =>
			s.name.toLowerCase().includes(query.toLowerCase()),
		);

		const all: Mentionable[] = [...users, ...roleSuggestions, ...special];
		return all.slice(0, 10).map((m) => ({
			id: `mentions-${m.id}`,
			label: m.name,
			user: m.user,
			onSelect: () => {},
		}));
	},
	resolveDisplayData(value, ctx) {
		if (value.startsWith("user-")) {
			const userId = value.replace("user-", "");
			const user = ctx.users.cache.get(userId);
			return {
				name: user?.name ?? value,
				user,
			};
		}
		if (value.startsWith("role-")) {
			const roleId = value.replace("role-", "");
			const role = [...ctx.roles.cache.values()].find((r) => r.id === roleId);
			return {
				name: role?.name ?? value,
			};
		}
		if (value === "everyone-room" || value === "everyone-thread") {
			return {
				name: value === "everyone-room" ? "@room" : "@thread",
			};
		}
		return { name: value };
	},
	toAST(node) {
		return {
			type: "mentions",
			value: node.attrs.id as string,
			name: node.attrs.name as string,
			negated: node.attrs.negated as boolean,
		};
	},
	toBackendQuery(ast) {
		const prefix = ast.negated ? "-" : "+";
		if (ast.value.startsWith("user-")) {
			return [
				`${prefix}metadata_fast.mentions_user:${ast.value.replace("user-", "")}`,
			];
		}
		if (ast.value.startsWith("role-")) {
			return [
				`${prefix}metadata_fast.mentions_role:${ast.value.replace("role-", "")}`,
			];
		}
		if (ast.value === "everyone-room" || ast.value === "everyone-thread") {
			return [`${prefix}metadata_fast.mentions_everyone:true`];
		}
		return [];
	},
	toPMNode(ast) {
		return schema.nodes.mentions.create({
			id: ast.value,
			name: ast.name ?? ast.value,
			negated: ast.negated,
		});
	},
};

// ---------------------------------------------------------------------------
// Registry map
// ---------------------------------------------------------------------------

export const SEARCH_FILTERS: Record<string, SearchFilterDef> = {
	author: authorFilter,
	channel: channelFilter,
	before: beforeFilter,
	after: afterFilter,
	has: hasFilter,
	pinned: pinnedFilter,
	mentions: mentionsFilter,
};

export const FILTER_NAMES = Object.keys(SEARCH_FILTERS);
