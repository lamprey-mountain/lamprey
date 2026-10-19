import { throttle } from "@solid-primitives/scheduled";
import type { RoomMember, User } from "sdk";
import { createMemo, createSignal, Show } from "solid-js";
import { useApi, useRoles, useRoomMembers } from "@/api";
import { createDropdown, type DropdownItem } from "@/atoms/Dropdown";
import { Avatar } from "@/avatar/UserAvatar";

export type UserRoleOption = {
	id: string;
	name: string;
	type: "Role" | "User";
	user?: User;
};

export function UserRoleSelect(props: {
	room_id: string;
	onSelect: (id: string, type: "Role" | "User") => void;
	selected?: string | null;
	excludeIds?: string[];
	types?: Array<"Role" | "User">;
	placeholder?: string;
	class?: string;
	mount?: Element | DocumentFragment | null;
}) {
	const api = useApi();
	const roles2 = useRoles();
	const roomMembers2 = useRoomMembers();

	const types = () => props.types ?? ["Role", "User"];

	const [memberResults, setMemberResults] = createSignal<{
		room_members: RoomMember[];
		users: User[];
	}>({ room_members: [], users: [] });

	const throttledSearch = throttle(async (q: string) => {
		if (!types().includes("User") || q.length === 0) {
			setMemberResults({ room_members: [], users: [] });
			return;
		}

		try {
			setMemberResults(await roomMembers2.search(props.room_id, q));
		} catch (e) {
			console.error("Member search failed:", e);
			setMemberResults({ room_members: [], users: [] });
		}
	}, 300);

	const options = createMemo<UserRoleOption[]>(() => {
		const exclude = new Set(props.excludeIds ?? []);
		const allowed = types();

		const roles: UserRoleOption[] = allowed.includes("Role")
			? [...roles2.cache.values()]
					.filter(
						(role) =>
							role.room_id === props.room_id &&
							role.id !== props.room_id && // exclude @everyone
							!exclude.has(role.id),
					)
					.map((role) => ({ id: role.id, name: role.name, type: "Role" }))
			: [];

		const users: UserRoleOption[] = [];
		if (allowed.includes("User")) {
			const seen = new Set<string>();
			const push = (id: string, name: string, user?: User) => {
				if (seen.has(id) || exclude.has(id)) return;
				seen.add(id);
				users.push({ id, name, type: "User", user });
			};

			// remote search results are already matched
			for (const member of memberResults().room_members) {
				const user = api.users.cache.get(member.user_id);
				push(
					member.user_id,
					member.override_name ?? user?.name ?? member.user_id,
					user,
				);
			}
			for (const user of memberResults().users) {
				push(user.id, user.name, user);
			}

			// cached room members
			for (const member of roomMembers2.listByRoom(props.room_id)) {
				const user = api.users.cache.get(member.user_id);
				push(
					member.user_id,
					member.override_name ?? user?.name ?? member.user_id,
					user,
				);
			}
		}

		return [...roles, ...users];
	});

	const dropdownOptions = createMemo<Array<DropdownItem<string>>>(() =>
		options().map((opt) => ({
			item: opt.id,
			label: opt.name,
			view: (
				<span class="user-role-option">
					<Show
						when={opt.user}
						fallback={<span class="user-role-option-avatar spacer" />}
					>
						{(user) => (
							<span class="user-role-option-avatar">
								<Avatar user={user()} />
							</span>
						)}
					</Show>
					<span class="user-role-option-name">{opt.name}</span>
				</span>
			),
		})),
	);

	const dropdown = createDropdown<string>({
		get selected() {
			return props.selected ?? undefined;
		},
		onInput: throttledSearch,
		onSelect: (id) => {
			if (!id) return;
			const opt = options().find((o) => o.id === id);
			if (opt) props.onSelect(opt.id, opt.type);
		},
		options: dropdownOptions,
		ignoreMissingLabel: true,
		get placeholder() {
			return props.placeholder;
		},
		get mount() {
			return props.mount ?? document.getElementById("overlay") ?? document.body;
		},
	});

	return <dropdown.View class={`user-role-select ${props.class ?? ""}`} />;
}
