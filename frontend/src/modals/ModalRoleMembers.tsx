import { throttle } from "@solid-primitives/scheduled";
import type { RoomMember } from "sdk";
import { createMemo, createSignal, For, Show } from "solid-js";
import { useApi } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Search } from "@/atoms/Search";
import { Avatar } from "@/components/shared/User";
import { useModals } from "@/contexts/modal";
import { Modal } from "./mod";

interface ModalRoleMembersProps {
	room_id: string;
	role_id: string;
}

export const ModalRoleMembers = (props: ModalRoleMembersProps) => {
	const api = useApi();
	const [, modalCtl] = useModals();
	const [search, setSearch] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<RoomMember[]>([]);
	const [overrides, setOverrides] = createSignal(new Map<string, boolean>());
	const [saving, setSaving] = createSignal(false);
	const [activeIndex, setActiveIndex] = createSignal(0);
	let listRef: HTMLUListElement | undefined;

	const role = () => api.roles.cache.get(props.role_id);

	const throttledSearch = throttle(async (query: string) => {
		if (!query) {
			setSearchResults([]);
			return;
		}
		try {
			const result = await api.room_members.search(props.room_id, query);
			setSearchResults(result.room_members);
		} catch (e) {
			console.error("Member search failed:", e);
			setSearchResults([]);
		}
	}, 300);

	const members = createMemo(() => {
		const byId = new Map<string, RoomMember>();
		for (const member of api.room_members.listByRoom(props.room_id)) {
			byId.set(member.user_id, member);
		}
		for (const member of searchResults()) {
			byId.set(member.user_id, member);
		}

		const query = search().toLowerCase();
		const list = [...byId.values()];
		if (!query) return list;

		return list.filter((member) => {
			const user = api.users.cache.get(member.user_id);
			const name = member.override_name ?? user?.name ?? member.user_id;
			return name.toLowerCase().includes(query);
		});
	});

	const activeItem = () =>
		Math.min(activeIndex(), Math.max(members().length - 1, 0));

	const focusItem = (index: number) => {
		setActiveIndex(index);
		const items = listRef?.querySelectorAll<HTMLElement>(
			".member-item .option",
		);
		items?.[index]?.focus();
	};

	const handleListKeyDown = (e: KeyboardEvent) => {
		const count = members().length;
		if (count === 0) return;

		const current = activeItem();
		let next: number;
		if (e.key === "ArrowDown") next = (current + 1) % count;
		else if (e.key === "ArrowUp") next = (current - 1 + count) % count;
		else if (e.key === "Home") next = 0;
		else if (e.key === "End") next = count - 1;
		else return;

		e.preventDefault();
		focusItem(next);
	};

	const isChecked = (member: RoomMember) =>
		overrides().get(member.user_id) ?? member.roles.includes(props.role_id);

	const setChecked = (user_id: string, checked: boolean) => {
		setOverrides((prev) => {
			const next = new Map(prev);
			next.set(user_id, checked);
			return next;
		});
	};

	const save = async () => {
		const changes = [...overrides()];
		if (changes.length === 0) {
			modalCtl.close();
			return;
		}

		setSaving(true);
		try {
			await Promise.all(
				changes.map(async ([user_id, checked]) => {
					const member = api.room_members.cache.get(
						`${props.room_id}:${user_id}`,
					);
					if ((member?.roles.includes(props.role_id) ?? false) === checked) {
						return;
					}
					if (checked) {
						await api.roles.addMember(props.room_id, props.role_id, user_id);
					} else {
						await api.roles.removeMember(
							props.room_id,
							props.role_id,
							user_id,
						);
					}
				}),
			);
			modalCtl.close();
		} catch (e) {
			console.error("Failed to update role members:", e);
			modalCtl.alert("Failed to update role members");
		} finally {
			setSaving(false);
		}
	};

	return (
		<Modal
			class="modal-role-members"
			onKeyDown={(e) => {
				if (e.key === "Escape") modalCtl.close();
			}}
		>
			<h3>members of {role()?.name ?? "role"}</h3>
			<Search
				placeholder="search members..."
				value={search}
				ref={(el) => queueMicrotask(() => el.focus())}
				onInput={(query) => {
					setSearch(query);
					throttledSearch(query);
				}}
			/>
			<ul class="members-list" ref={listRef} onKeyDown={handleListKeyDown}>
				<For each={members()}>
					{(member, index) => {
						const user = api.users.use(() => member.user_id);
						const name = () =>
							member.override_name ?? user()?.name ?? member.user_id;
						const id = `role-member-${props.role_id}-${member.user_id}`;
						return (
							<li
								class="member-item"
								onFocusIn={() => setActiveIndex(index())}
							>
								<CheckboxOption
									id={id}
									checked={isChecked(member)}
									onChange={(checked) => setChecked(member.user_id, checked)}
									seed={id}
									tabIndex={index() === activeItem() ? 0 : -1}
								>
									<Checkbox checked={isChecked(member)} seed={id} />
									<Avatar user={user()} pad={4} />
									<label for={id}>{name()}</label>
								</CheckboxOption>
							</li>
						);
					}}
				</For>
				<Show when={members().length === 0}>
					<li class="empty">no members found</li>
				</Show>
			</ul>
			<div class="bottom">
				<button
					type="button"
					class="button"
					onClick={() => modalCtl.close()}
					disabled={saving()}
				>
					cancel
				</button>
				<button
					type="button"
					class="button primary"
					onClick={save}
					disabled={saving()}
				>
					{saving() ? "saving..." : "save"}
				</button>
			</div>
		</Modal>
	);
};
