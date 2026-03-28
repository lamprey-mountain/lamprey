import { createEffect, createMemo, createSignal } from "solid-js";
import { useAutocomplete } from "../contexts/autocomplete";
import {
	useApi2,
	useChannels2,
	useEmoji2,
	useRoles2,
	useRoomMembers2,
	useThreadMembers2,
	useUsers2,
} from "@/api";
import { go } from "fuzzysort";
import { type Channel, type EmojiCustom, type User } from "sdk";
import type { Role } from "sdk";
import { type Command, useSlashCommands } from "../contexts/slash-commands";
import { type EmojiData, emojiResource } from "../emoji";
import { usePermissions } from "./usePermissions";
import { useCurrentUser } from "../contexts/currentUser";
import type {
	AutocompleteItem,
	AutocompleteMentionItem,
} from "../contexts/autocomplete";

export const useAutocompleteData = () => {
	const api2 = useApi2();
	const channels2 = useChannels2();
	const store = useApi2();
	const rolesApi = useRoles2();
	const threadMembers2 = useThreadMembers2();
	const roomMembers2 = useRoomMembers2();
	const users2 = useUsers2();
	const emoji2 = useEmoji2();
	const currentUser = useCurrentUser();
	const { state, setResults } = useAutocomplete();

	// Get permissions for @everyone/@room mentions
	const channelForPerms = () => {
		if (state.kind?.type === "mention") {
			return channels2.cache.get(state.kind.channelId);
		}
		return null;
	};
	const perms = usePermissions(
		() => currentUser()?.id ?? "",
		() => channelForPerms()?.room_id ?? undefined,
		() => state.kind?.type === "mention" ? (state.kind as any).channelId : "",
	);
	const hasMassMention = () => perms.has("MessageMassMention");

	const [allUsers, setAllUsers] = createSignal<User[]>([]);
	const [allChannels, setAllChannels] = createSignal<Channel[]>([]);
	const [allEmoji, setAllEmoji] = createSignal<(EmojiCustom | EmojiData)[]>([]);
	const [allCommands, setAllCommands] = createSignal<Command[]>([]);
	const [allRoles, setAllRoles] = createSignal<Role[]>([]);

	// Fetch data based on autocomplete type
	createEffect(() => {
		const kind = state.kind;
		if (!kind) return;

		if (kind.type === "mention") {
			const channel = channels2.cache.get(kind.channelId);
			const roomId = kind.roomId ?? channel?.room_id;

			const threadMembers = threadMembers2.useList(() => kind.channelId)();
			const roomMembers = roomId
				? roomMembers2.useList(() => roomId)()
				: undefined;

			const userIds = new Set<string>();
			// Access ids from PaginatedList state and fetch members from cache
			threadMembers?.state.ids.forEach((id: string) => {
				const member = threadMembers2.cache.get(id);
				if (member?.user_id) userIds.add(member.user_id);
			});
			roomMembers?.state.ids.forEach((id: string) => {
				const member = roomMembers2.cache.get(id);
				if (member?.user_id) userIds.add(member.user_id);
			});

			// Build user list from cache or use member data as fallback
			const users = [...userIds].map((id) => {
				const cachedUser = users2.cache.get(id);
				if (cachedUser && cachedUser.id) {
					return cachedUser;
				}
				// Fallback: create a minimal user object from the member data
				// Find the member to get any available name info
				const threadMember = threadMembers?.state.ids
					.map((id: string) => threadMembers2.cache.get(id))
					.find((m) => m?.user_id === id);
				const roomMember = roomMembers?.state.ids
					.map((id: string) => roomMembers2.cache.get(id))
					.find((m) => m?.user_id === id);
				const member = threadMember || roomMember;
				// override_name only exists on RoomMember, not ThreadMember
				const name = "override_name" in (member ?? {})
					? (member as any)?.override_name
					: undefined;
				return {
					id: id,
					name: name || id,
				} as User;
			});
			setAllUsers(users);

			// Also fetch mentionable roles for combined autocomplete
			if (roomId) {
				const mentionableRoles = [...rolesApi.cache.values()].filter(
					(r) => r.room_id === roomId && r.is_mentionable && r.id !== roomId,
				);
				setAllRoles(mentionableRoles);
			}
		} else if (kind.type === "channel") {
			const channel = channels2.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const channels = [...channels2.cache.values()].filter(
				(c) => c.type !== "Category" && c.room_id === roomId,
			);
			setAllChannels(channels);
		} else if (kind.type === "emoji") {
			const channel = channels2.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const combined: (EmojiCustom | EmojiData)[] = [];
			if (roomId) {
				// Get custom emoji from cache for this room
				const roomEmoji = [...emoji2.cache.values()].filter(
					(e) => e.owner?.owner === "Room" && e.owner.room_id === roomId,
				);
				combined.push(...roomEmoji);
			}
			const unicodeEmoji = emojiResource();
			if (unicodeEmoji) {
				combined.push(...unicodeEmoji);
			}
			setAllEmoji(combined);
		} else if (kind.type === "command") {
			const slashCommands = useSlashCommands();
			const allCommands = slashCommands.getAll();
			const channel = channels2.cache.get(kind.channelId);

			const filteredCommands = allCommands.filter((cmd) => {
				if (cmd.canUse) {
					return cmd.canUse(
						api2,
						channels2,
						channel?.room_id ?? undefined,
						channel!,
						store,
					);
				}
				return true;
			});

			setAllCommands(filteredCommands);
		}
	});

	// Filter results based on query
	const filtered = createMemo(() => {
		const kind = state.kind;
		if (!kind) return [];

		const query = state.query;
		const type = kind.type;

		if (type === "mention") {
			// Combined users, roles, and @everyone search using fuzzysort
			const users = allUsers();
			const roles = allRoles();

			const results: AutocompleteMentionItem[] = [];

			// Use fuzzysort for users
			const userResults = go(query, users, {
				key: "name",
				limit: 10,
				all: true,
			});
			for (const result of userResults) {
				results.push({
					type: "user" as const,
					user_id: result.obj.id,
					name: result.obj.name,
					user: result.obj,
				});
			}

			// Use fuzzysort for roles
			const roleResults = go(query, roles, {
				key: "name",
				limit: 10,
				all: true,
			});
			for (const result of roleResults) {
				results.push({
					type: "role" as const,
					role_id: result.obj.id,
					name: result.obj.name,
				});
			}

			// Add @everyone if has permission and query matches
			if (hasMassMention() && "everyone".startsWith(query.toLowerCase())) {
				results.push({ type: "everyone" as const, mention_type: "everyone" });
			}

			// Limit to 10 results total
			const limited = results.slice(0, 10);

			// Convert to fuzzysort-like results
			return limited.map((item) => ({
				obj: item,
				score: 0,
				hits: [{
					value: item.type === "everyone" ? "@everyone" : item.name,
				}],
			}));
		} else if (type === "channel") {
			const results = go(query, allChannels(), {
				key: "name",
				limit: 10,
				all: true,
			});
			return results;
		} else if (type === "emoji") {
			// Normalize emoji for search - custom emoji use 'name', unicode use 'label'
			const normalizedEmoji = allEmoji().map((e) => ({
				...e,
				searchLabel: "label" in e ? e.label : e.name,
			}));
			const results = go(query, normalizedEmoji, {
				keys: ["searchLabel", "shortcodes"],
				limit: 10,
				all: true,
			}) as any;
			return results.map((r: any) => ({
				obj: {
					type: "emoji" as const,
					id: r.obj.id,
					name: "name" in r.obj ? r.obj.name : "",
					char: "char" in r.obj ? r.obj.char : undefined,
				},
				score: r.score,
				hits: r.hits,
			}));
		} else if (type === "command") {
			const results = go(query, allCommands(), {
				key: "name",
				limit: 10,
				all: true,
			}) as any;
			return results.map((r: any) => ({
				obj: {
					type: "command" as const,
					command: r.obj.name,
				},
				score: r.score,
				hits: r.hits,
			}));
		}

		return [];
	});

	// NOTE: this is kind of ugly, maybe i should remove it?
	createEffect(() => {
		setResults(filtered().map((i: any) => i.obj as AutocompleteItem));
	});

	return {
		filtered,
		allUsers,
		allChannels,
		allEmoji,
		allCommands,
		allRoles,
	};
};
