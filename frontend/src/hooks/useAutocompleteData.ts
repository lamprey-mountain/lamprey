import { go } from "fuzzysort";
import type {
	Channel,
	EmojiCustom,
	Role,
	RoomMember,
	ThreadMember,
	User,
} from "sdk";
import { createEffect, createMemo, createSignal } from "solid-js";
import {
	useApi,
	useChannels,
	useEmoji,
	useRoles,
	useRoomMembers,
	useThreadMembers,
	useUsers,
} from "@/api";
import type {
	AutocompleteItem,
	AutocompleteMentionItem,
} from "@/contexts/autocomplete";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useCurrentUser } from "@/contexts/currentUser";
import { type Command, useSlashCommands } from "@/contexts/slash-commands";
import { type EmojiData, emojiResource } from "@/lib/emoji";
import { usePermissions } from "./usePermissions";

type AutocompleteSearchResult = {
	obj: AutocompleteItem;
	score: number;
	hits: Array<{ value: string }>;
};

export const useAutocompleteData = () => {
	const api2 = useApi();
	const channels2 = useChannels();
	const store = useApi();
	const rolesApi = useRoles();
	const threadMembers2 = useThreadMembers();
	const roomMembers2 = useRoomMembers();
	const users2 = useUsers();
	const emoji2 = useEmoji();
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
		() => channelForPerms()?.id ?? "",
	);
	const hasMassMention = () => perms.has("MessageMassMention");

	const [allUsers, setAllUsers] = createSignal<User[]>([]);
	const [allChannels, setAllChannels] = createSignal<Channel[]>([]);
	const [allEmoji, setAllEmoji] = createSignal<(EmojiCustom | EmojiData)[]>([]);
	const [allCommands, setAllCommands] = createSignal<Command[]>([]);
	const [allRoles, setAllRoles] = createSignal<Role[]>([]);

	const threadMembersResource = threadMembers2.useList(() =>
		state.kind?.type === "mention" ? state.kind.channelId : undefined,
	);

	const roomMembersResource = roomMembers2.useList(() => {
		if (state.kind?.type !== "mention") return;
		const channel = channels2.cache.get(state.kind.channelId);
		return state.kind.roomId ?? channel?.room_id ?? undefined;
	});

	const channel = () => channels2.get(state.kind?.channelId ?? "");

	// Fetch data based on autocomplete type
	createEffect(() => {
		const kind = state.kind;
		if (!kind) return;

		if (kind.type === "mention") {
			const threadMembers = threadMembersResource();
			const roomMembers = roomMembersResource();

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
				if (cachedUser?.id) {
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
				const member: RoomMember | ThreadMember | undefined =
					threadMember || roomMember;
				// override_name only exists on RoomMember, not ThreadMember
				const name =
					member && "override_name" in member
						? ((member as RoomMember).override_name ?? undefined)
						: undefined;
				return {
					id: id,
					name: name || id,
				} as User;
			});
			setAllUsers(users);

			// Also fetch mentionable roles for combined autocomplete
			const roomId = kind.roomId ?? channel()?.room_id;
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
	const filtered = createMemo((): AutocompleteSearchResult[] => {
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
			return limited.map(
				(item): AutocompleteSearchResult => ({
					obj: item,
					score: 0,
					hits: [
						{
							value: item.type === "everyone" ? "@everyone" : item.name,
						},
					],
				}),
			);
		} else if (type === "channel") {
			const results = go(query, allChannels(), {
				key: "name",
				limit: 10,
				all: true,
			});
			return results.map(
				(r): AutocompleteSearchResult => ({
					obj: {
						type: "channel" as const,
						channel: r.obj,
						channel_id: r.obj.id,
						name: r.obj.name,
					},
					score: r.score,
					hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
				}),
			);
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
			});
			return results.map(
				(r): AutocompleteSearchResult => ({
					obj: {
						type: "emoji" as const,
						id: r.obj.id,
						name: "name" in r.obj ? r.obj.name : "",
						char: "char" in r.obj ? r.obj.char : undefined,
					},
					score: r.score,
					hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
				}),
			);
		} else if (type === "command") {
			const results = go(query, allCommands(), {
				key: "name",
				limit: 10,
				all: true,
			});
			return results.map(
				(r): AutocompleteSearchResult => ({
					obj: {
						type: "command" as const,
						command: r.obj.name,
						id: r.obj.id,
						description: r.obj.description,
					},
					score: r.score,
					hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
				}),
			);
		}

		return [];
	});

	// NOTE: this is kind of ugly, maybe i should remove it?
	createEffect(() => {
		const results = filtered().map((i) => i.obj);
		// Use untrack if setResults ends up touching parts of 'state'
		// that the logic above depends on
		setResults(results);
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
