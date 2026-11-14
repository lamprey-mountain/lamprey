import { go } from "fuzzysort";
import type {
	Channel,
	EmojiCustom,
	Role,
	RoomMember,
	ThreadMember,
	User,
} from "sdk";
import { createEffect, createMemo, createResource, createSignal } from "solid-js";
import { useApi } from "@/api";
import type {
	AutocompleteItem,
	AutocompleteMentionItem,
} from "@/contexts/autocomplete";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useCurrentUser } from "@/contexts/currentUser";
import { useSlashCommands } from "@/contexts/slash-commands";
import { durationSuggestions } from "@/lib/commands/duration";
import type { Command } from "@/lib/commands/types";
import { type EmojiData, emojiResource } from "@/lib/emoji";
import { usePermissions } from "./usePermissions";

type AutocompleteSearchResult = {
	obj: AutocompleteItem;
	score: number;
	hits: Array<{ value: string }>;
};

function shortestName(shortcodes: string[], fallback: string): string {
	let best = fallback;
	for (const shortcode of shortcodes) {
		if (shortcode.length >= 3 && shortcode.length < best.length) {
			best = shortcode;
		}
	}
	return best;
}

export const useAutocompleteData = () => {
	const api = useApi();
	const currentUser = useCurrentUser();
	const { state, setResults } = useAutocomplete();

	// Get permissions for @everyone/@room mentions
	const channelForPerms = () => {
		const kind = state.kind;
		if (kind?.type === "mention" || kind?.type === "commandArg") {
			return api.channels.cache.get(kind.channelId);
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

	const threadMembersResource = api.threadMembers.useList(() => {
		const kind = state.kind;
		return kind?.type === "mention" || kind?.type === "commandArg"
			? kind.channelId
			: undefined;
	});

	const roomMembersResource = api.roomMembers.useList(() => {
		const kind = state.kind;
		if (kind?.type !== "mention" && kind?.type !== "commandArg") return;
		const channel = api.channels.cache.get(kind.channelId);
		return kind.roomId ?? channel?.room_id ?? undefined;
	});

	const [emojiRoomsResource] = createResource(
		() => {
			if (state.kind?.type !== "emoji") return;

			// pass every room: rooms without emoji are used to drop stale cached emoji
			return [...api.rooms.cache.values()];
		},
		async (rooms) => {
			await api.emoji.loadCached();
			await api.emoji.listAllCustom(rooms);
		},
	);

	const channel = () => api.channels.get(state.kind?.channelId ?? "");

	const collectUsers = () => {
		const threadMembers = threadMembersResource();
		const roomMembers = roomMembersResource();

		const userIds = new Set<string>();
		// Access ids from PaginatedList state and fetch members from cache
		threadMembers?.state.ids.forEach((id: string) => {
			const member = api.threadMembers.cache.get(id);
			if (member?.user_id) userIds.add(member.user_id);
		});
		roomMembers?.state.ids.forEach((id: string) => {
			const member = api.roomMembers.cache.get(id);
			if (member?.user_id) userIds.add(member.user_id);
		});

		// Build user list from cache or use member data as fallback
		return [...userIds].map((id) => {
			const cachedUser = api.users.cache.get(id);
			if (cachedUser?.id) {
				return cachedUser;
			}
			// Fallback: create a minimal user object from the member data
			// Find the member to get any available name info
			const threadMember = threadMembers?.state.ids
				.map((id: string) => api.threadMembers.cache.get(id))
				.find((m) => m?.user_id === id);
			const roomMember = roomMembers?.state.ids
				.map((id: string) => api.roomMembers.cache.get(id))
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
	};

	// Fetch data based on autocomplete type
	createEffect(() => {
		const kind = state.kind;
		if (!kind) return;

		if (kind.type === "mention" || kind.type === "commandArg") {
			if (kind.type === "mention" || kind.option.kind === "user") {
				setAllUsers(collectUsers());
			}

			if (kind.type === "mention") {
				// Also fetch mentionable roles for combined autocomplete
				const roomId = kind.roomId ?? channel()?.room_id;
				if (roomId) {
					const mentionableRoles = [...api.roles.cache.values()].filter(
						(r) => r.room_id === roomId && r.is_mentionable && r.id !== roomId,
					);
					setAllRoles(mentionableRoles);
				}
			}
		} else if (kind.type === "channel") {
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const channels = [...api.channels.cache.values()].filter(
				(c) => c.type !== "Category" && c.room_id === roomId,
			);
			setAllChannels(channels);
		} else if (kind.type === "emoji") {
			// rerun once cached emoji have been loaded
			emojiRoomsResource();

			const combined: (EmojiCustom | EmojiData)[] = [];
			for (const emoji of api.emoji.cache.values()) {
				if (emoji.owner?.owner !== "Room") continue;

				const room = api.rooms.cache.get(emoji.owner.room_id);
				if (!room || room.emoji_count === 0) continue;

				combined.push(emoji);
			}
			const unicodeEmoji = emojiResource();
			if (unicodeEmoji) {
				combined.push(...unicodeEmoji.values());
			}
			setAllEmoji(combined);
		} else if (kind.type === "command") {
			const slashCommands = useSlashCommands();
			const allCommands = slashCommands.getAll();
			const channel = api.channels.cache.get(kind.channelId);
			if (!channel) return;

			const filteredCommands = allCommands.filter((cmd) => {
				return cmd.canUse(api, channel);
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
						name: r.obj.name ?? r.obj.auto_name ?? "",
					},
					score: r.score,
					hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
				}),
			);
		} else if (type === "emoji") {
			// Normalize emoji for search
			const normalizedEmoji = allEmoji().map((e) => {
				let room_name: string | undefined;
				if (!("hexcode" in e) && e.owner?.owner === "Room") {
					room_name = api.rooms.cache.get(e.owner.room_id)?.name;
				}

				return {
					...e,
					searchLabel: "label" in e ? e.label : e.name,
					id: "hexcode" in e ? e.hexcode : e.id,
					room_name,
				};
			});
			const results = go(query, normalizedEmoji, {
				keys: ["searchLabel"],
				limit: 10,
				all: true,
			});
			return results.map((r): AutocompleteSearchResult => {
				const name = "name" in r.obj ? r.obj.name : r.obj.label;
				const short_name =
					"shortcodes" in r.obj
						? shortestName(r.obj.shortcodes, name)
						: undefined;

				return {
					obj: {
						type: "emoji" as const,
						id: r.obj.id,
						name,
						short_name,
						char: "char" in r.obj ? r.obj.char : undefined,
						room_name: r.obj.room_name,
					},
					score: r.score,
					hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
				};
			});
		} else if (kind.type === "commandArg") {
			if (kind.option.kind === "user") {
				const results = go(query, allUsers(), {
					key: "name",
					limit: 10,
					all: true,
				});
				return results.map(
					(r): AutocompleteSearchResult => ({
						obj: {
							type: "user" as const,
							user_id: r.obj.id,
							name: r.obj.name,
							user: r.obj,
						},
						score: r.score,
						hits: (r as { hits?: Array<{ value: string }> }).hits ?? [],
					}),
				);
			}

			if (kind.option.kind === "duration") {
				return durationSuggestions(query).map(
					(s): AutocompleteSearchResult => ({
						obj: {
							type: "arg" as const,
							value: s.value,
							label: s.label,
							description: s.description,
						},
						score: 0,
						hits: [{ value: s.label }],
					}),
				);
			}

			return [];
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
