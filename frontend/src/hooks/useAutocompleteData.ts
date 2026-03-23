import { createEffect, createMemo, createSignal } from "solid-js";
import { useAutocomplete } from "../contexts/autocomplete";
import { useApi, useApi2, useRoles2 } from "../api";
import { go } from "fuzzysort";
import { type Channel, type EmojiCustom, type User } from "sdk";
import type { Role } from "sdk";
import { type Command, useSlashCommands } from "../contexts/slash-commands";
import { type EmojiData, emojiResource } from "../emoji";
import { usePermissions } from "./usePermissions";
import { useCurrentUser } from "../contexts/currentUser";
import type { AutocompleteMentionItem } from "../contexts/autocomplete";

export const useAutocompleteData = () => {
	const api = useApi();
	const store = useApi2();
	const rolesApi = useRoles2();
	const currentUser = useCurrentUser();
	const { state, setResults } = useAutocomplete();

	// Get permissions for @everyone/@room mentions
	const channelForPerms = () => {
		if (state.kind?.type === "mention") {
			return api.channels.cache.get(state.kind.channelId);
		}
		return null;
	};
	const perms = usePermissions(
		() => currentUser()?.id ?? "",
		() => channelForPerms()?.room_id,
		() => state.kind?.type === "mention" ? state.kind.channelId : "",
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
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = kind.roomId ?? channel?.room_id;

			const threadMembers = api.thread_members.list(() => kind.channelId)();
			const roomMembers = roomId
				? api.room_members.list(() => roomId)()
				: undefined;

			const userIds = new Set<string>();
			threadMembers?.items.forEach((m) => userIds.add(m.user_id));
			roomMembers?.items.forEach((m) => userIds.add(m.user_id));

			// Build user list from cache or use member data as fallback
			const users = [...userIds].map((id) => {
				const cachedUser = api.users.cache.get(id);
				if (cachedUser && cachedUser.id) {
					return cachedUser;
				}
				// Fallback: create a minimal user object from the member data
				// Find the member to get any available name info
				const member = threadMembers?.items.find((m) => m.user_id === id) ||
					roomMembers?.items.find((m) => m.user_id === id);
				return {
					id: id,
					name: member?.override_name || id,
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
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const channels = [...api.channels.cache.values()].filter(
				(c) => c.type !== "Category" && c.room_id === roomId,
			);
			setAllChannels(channels);
		} else if (kind.type === "emoji") {
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const combined: (EmojiCustom | EmojiData)[] = [];
			if (roomId) {
				// Get custom emoji from cache for this room
				const roomEmoji = [...api.emoji.cache.values()].filter(
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
			const channel = api.channels.cache.get(kind.channelId);

			const filteredCommands = allCommands.filter((cmd) => {
				if (cmd.canUse) {
					return cmd.canUse(
						api,
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
			return results;
		} else if (type === "command") {
			const results = go(query, allCommands(), {
				key: "name",
				limit: 10,
				all: true,
			}) as any;
			return results;
		}

		return [] as any;
	});

	// NOTE: this is kind of ugly, maybe i should remove it?
	createEffect(() => {
		setResults(filtered().map((i) => i.obj));
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
