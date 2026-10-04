import { type ReferenceElement, shift } from "@floating-ui/dom";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { throttle } from "@solid-primitives/scheduled";
import type { Role, RoomMember, RoomMemberOrigin, User } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { Search } from "@/atoms/Search";
import { Time } from "@/atoms/Time.tsx";
import { Avatar } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useMenu } from "@/contexts/mod.tsx";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions.ts";
import type { RoomT } from "@/types";
import { getDate } from "@/utils/general";

export function Members(props: VoidProps<{ room: RoomT }>) {
	const { setMenu } = useMenu();
	const api = useApi();

	// Get member IDs for this room from cache
	const memberIds = createMemo(() => {
		const ids: string[] = [];
		for (const [key] of api.roomMembers.cache.entries()) {
			if (key.startsWith(`${props.room.id}:`)) {
				ids.push(key);
			}
		}
		return ids;
	});

	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	const removeRole = (user_id: string, role_id: string) => () => {
		const [, modalCtl] = useModals();
		modalCtl.confirm("really remove?", (conf) => {
			if (!conf) return;
			api.client.http.DELETE(
				"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
				{ params: { path: { room_id: props.room.id, role_id, user_id } } },
			);
		});
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) {
					// Trigger a re-fetch by accessing the cache
					api.roomMembers.cache.size;
				}
			}
		},
	);

	const [editRoles, setEditRoles] = createSignal<{
		member: RoomMember;
		x: number;
		y: number;
	}>();

	const [query, setQuery] = createSignal("");
	const [searchResults, setSearchResults] = createSignal<string[]>([]);

	const throttledSearch = throttle(async (q: string) => {
		if (q.length > 0) {
			const results = await api.room_members.search(props.room.id, q);
			if (results) {
				setSearchResults(results.users.map((i) => `${props.room.id}:${i.id}`));
			} else {
				setSearchResults([]);
			}
		} else {
			setSearchResults([]);
		}
	}, 500);

	createEffect(() => {
		throttledSearch(query());
	});

	return (
		<div class="room-settings-members">
			<h2>Members</h2>
			<Search
				placeholder="Search users..."
				onInput={setQuery}
				ref={(el) => queueMicrotask(() => el.focus())}
			/>
			<header>
				<div class="name">name</div>
				<div class="joined">joined</div>
			</header>
			<Show
				when={
					query().length > 0
						? searchResults().length > 0
						: memberIds().length > 0
				}
			>
				<ul>
					<For each={query().length > 0 ? searchResults() : memberIds()}>
						{(id) => {
							const i = api.roomMembers.cache.get(id);
							if (!i) return null;
							const user = api.users.use(() => i.user_id);
							const name = () => i.override_name ?? user()?.name;
							return (
								<li>
									<div class="profile">
										<Avatar user={user()} />
										<div>
											<h3 class="name">{name()}</h3>
											<ul class="roles">
												<For each={i.roles}>
													{(role_id) => {
														const role = api.roles.cache.get(role_id);
														return (
															<li>
																<button
																	type="button"
																	class="button"
																	onClick={removeRole(i.user_id, role_id)}
																>
																	{role?.name ?? "unknown role"}
																</button>
															</li>
														);
													}}
												</For>
												<li class="add">
													<button
														type="button"
														class="button"
														onClick={(e) => {
															e.stopImmediatePropagation();
															setEditRoles({
																member: i,
																x: e.clientX,
																y: e.clientY,
															});
														}}
													>
														<em>add role...</em>
													</button>
												</li>
											</ul>
										</div>
									</div>
									{/* TODO
									<div class="notes">
										{i.deaf && <div>deaf</div>}
										{i.mute && <div>mute</div>}
									</div> */}
									<div class="joined">
										<Time date={getDate(i.joined_at)} />
										<div class="dim">{formatOrigin(i.origin)}</div>
									</div>
									<div style="flex:1"></div>
									<button
										type="button"
										class="button"
										onClick={(e) => {
											queueMicrotask(() => {
												setMenu({
													type: "user",
													room_id: props.room.id,
													user_id: i.user_id,
													x: e.clientX,
													y: e.clientY,
													admin: true,
												});
											});
										}}
									>
										options
									</button>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
			<Show when={editRoles()}>
				{(ed) => (
					<EditRoles
						x={ed().x}
						y={ed().y}
						user_id={ed().member.user_id}
						room={props.room}
					/>
				)}
			</Show>
		</div>
	);
}

// TODO: make this an actual context menu?
const EditRoles = (props: {
	x: number;
	y: number;
	user_id: string;
	room: RoomT;
}) => {
	const api = useApi();
	const member = api.roomMembers.cache.get(`${props.room.id}:${props.user_id}`);
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();

	createEffect(() => {
		setMenuParentRef({
			getBoundingClientRect: () => ({
				x: props.x,
				y: props.y,
				left: props.x,
				top: props.y,
				right: props.x,
				bottom: props.y,
				width: 0,
				height: 0,
			}),
		});

		props.x;
		props.y;
	});

	const menuFloating = useFloating(
		() => menuParentRef(),
		() => menuRef(),
		{
			middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
			placement: "right-start",
		},
	);

	const handleChecked =
		(r: Role) => (e: InputEvent & { target: HTMLInputElement }) => {
			const role_id = r.id;
			const user_id = member?.user_id;
			if (!user_id) return;
			if (e.target?.checked) {
				api.client.http.PUT(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{
						params: {
							path: {
								room_id: props.room.id,
								role_id,
								user_id,
							},
						},
					},
				);
			} else {
				api.client.http.DELETE(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{
						params: {
							path: {
								room_id: props.room.id,
								role_id,
								user_id,
							},
						},
					},
				);
			}
		};

	const getRoles = () =>
		[...api.roles.cache.values()].filter(
			(r) => r.room_id === props.room.id && r.id !== r.room_id,
		);

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;

	const { permissions } = usePermissions(
		self_id,
		() => props.room.id,
		() => undefined,
	);

	return (
		<menu
			class="edit-roles"
			style={{
				translate: `${menuFloating.x}px ${menuFloating.y}px`,
			}}
			ref={setMenuRef}
			onClick={(e) => e.stopImmediatePropagation()}
		>
			<For each={getRoles()}>
				{(r) => {
					const memberRoles = member?.roles ?? [];
					return (
						<label classList={{ disabled: r.position >= permissions().rank }}>
							<input
								type="checkbox"
								checked={memberRoles.includes(r.id)}
								onInput={handleChecked(r)}
								disabled={r.position >= permissions().rank}
							/>
							<div>
								<div classList={{ has: memberRoles.includes(r.id) }}>
									{r.name}
								</div>
								<div class="dim">{r.description}</div>
							</div>
						</label>
					);
				}}
			</For>
		</menu>
	);
};

function formatOrigin(o: RoomMemberOrigin | undefined | null) {
	switch (o?.type) {
		case "Invite":
			return o.code;
		case "BotInstall":
			return "bot install";
		case "Bridged":
			return "bridged";
		case "Creator":
			return "room creator";
		case null:
		case undefined:
			return "unknown";
	}
}
