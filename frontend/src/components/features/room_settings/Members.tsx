import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { throttle } from "@solid-primitives/scheduled";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Resizable } from "@/atoms/Resizable";
import { Search } from "@/atoms/Search";
import { createTooltip } from "@/atoms/Tooltip";
import {
	hasModerationFlags,
	MemberFlags,
} from "@/components/shared/MemberFlags";
import { Avatar } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useMenu } from "@/contexts/mod.tsx";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions.ts";
import type { RoomT } from "@/types";
import { icMore } from "@/utils/icons";
import { MemberPanel } from "./MemberPanel";

export function Members(props: VoidProps<{ room: RoomT }>) {
	const { setMenu } = useMenu();
	const api = useApi();
	const [, modalCtl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room.id,
		() => undefined,
	);

	const moreTooltip = createTooltip({ tip: () => "More" });

	const list = api.roomMembers.useList(() => props.room.id);

	const cachedIds = () => {
		const ids: string[] = [];
		for (const [key] of api.roomMembers.cache.entries()) {
			if (key.startsWith(`${props.room.id}:`)) ids.push(key);
		}
		return ids;
	};

	// use the paginated list once it's ready, but show cached members until then
	const memberIds = createMemo(() => {
		const name = (key: string) => {
			const member = api.roomMembers.cache.get(key);
			const user = member && api.users.cache.get(member.user_id);
			return (member?.override_name ?? user?.name ?? key).toLowerCase();
		};

		const ids = new Set([...(list()?.state.ids ?? []), ...cachedIds()]);
		return [...ids].sort((a, b) => name(a).localeCompare(name(b)));
	});

	const removeRole = (user_id: string, role_id: string) => () => {
		if (!hasPermission("RoleManage")) return;
		modalCtl.confirm("really remove?", (conf) => {
			if (!conf) return;
			api.client.http.DELETE(
				"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
				{ params: { path: { room_id: props.room.id, role_id, user_id } } },
			);
		});
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	// keep fetching while the sentinel is visible (e.g. when a page is too
	// short to push the sentinel out of the viewport)
	createEffect(() => {
		const roomList = list();
		const el = bottom();
		if (!roomList || !el || !el.isConnected) return;
		if (!roomList.state.has_more || roomList.state.isLoading) return;
		if (el.getBoundingClientRect().top > window.innerHeight) return;
		api.roomMembers.fetchNextPage(props.room.id);
	});

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			if (!entries.some((entry) => entry.isIntersecting)) return;
			const roomList = list();
			if (!roomList || !roomList.state.has_more || roomList.state.isLoading)
				return;
			api.roomMembers.fetchNextPage(props.room.id);
		},
	);

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

	const visibleIds = () => (query().length > 0 ? searchResults() : memberIds());

	const [focusedIndex, setFocusedIndex] = createSignal(0);
	let listRef: HTMLUListElement | undefined;

	const activeIndex = () =>
		Math.min(focusedIndex(), Math.max(visibleIds().length - 1, 0));

	const focusRow = (index: number) => {
		const rows = listRef?.querySelectorAll<HTMLElement>("[data-member-row]");
		if (!rows || rows.length === 0) return;
		const next = (index + rows.length) % rows.length;
		setFocusedIndex(next);
		rows[next].focus();
	};

	const handleListKeyDown = (e: KeyboardEvent) => {
		if (visibleIds().length === 0) return;
		if (e.key === "ArrowDown") {
			e.preventDefault();
			focusRow(activeIndex() + 1);
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			focusRow(activeIndex() - 1);
		} else if (e.key === "Home") {
			e.preventDefault();
			focusRow(0);
		} else if (e.key === "End") {
			e.preventDefault();
			focusRow(visibleIds().length - 1);
		}
	};

	const [selectedUserId, setSelectedUserId] = createSignal<string | null>(null);

	const toggleSelected = (user_id: string) => {
		setSelectedUserId((current) => (current === user_id ? null : user_id));
	};

	return (
		<div class="room-settings-members">
			<div class="member-main">
				<h2>Members</h2>
				<Search
					placeholder="Search users..."
					onInput={setQuery}
					ref={(el) => queueMicrotask(() => el.focus())}
				/>
				<Show
					when={visibleIds().length > 0}
					fallback={
						<div class="dim empty">
							{query().length > 0 ? "no members found" : "loading..."}
						</div>
					}
				>
					<ul ref={listRef} onKeyDown={handleListKeyDown}>
						<For each={visibleIds()}>
							{(id, index) => {
								const i = api.roomMembers.cache.get(id);
								if (!i) return null;
								const user = api.users.use(() => i.user_id);
								const name = () => i.override_name ?? user()?.name;
								return (
									<li
										class="menu-user"
										classList={{ selected: selectedUserId() === i.user_id }}
										data-member-row
										data-user-id={i.user_id}
										data-room-id={props.room.id}
										tabIndex={index() === activeIndex() ? 0 : -1}
										onFocus={() => setFocusedIndex(index())}
										onClick={(e) => {
											if (
												(e.target as HTMLElement).closest("button, a, input")
											) {
												return;
											}
											toggleSelected(i.user_id);
										}}
										onKeyDown={(e) => {
											if (e.key !== "Enter" && e.key !== " ") return;
											if (e.target !== e.currentTarget) return;
											e.preventDefault();
											toggleSelected(i.user_id);
										}}
									>
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
																		title={
																			hasPermission("RoleManage")
																				? "click to remove this role"
																				: undefined
																		}
																		disabled={!hasPermission("RoleManage")}
																		onClick={removeRole(i.user_id, role_id)}
																		onContextMenu={(e) => {
																			e.preventDefault();
																			e.stopPropagation();
																			setMenu({
																				type: "role",
																				role_id,
																				room_id: props.room.id,
																				x: e.clientX,
																				y: e.clientY,
																			});
																		}}
																	>
																		{role?.name ?? "unknown role"}
																	</button>
																</li>
															);
														}}
													</For>
													<Show when={hasPermission("RoleManage")}>
														<li class="add">
															<button
																type="button"
																class="button"
																onClick={(e) => {
																	e.stopPropagation();
																	setMenu({
																		type: "member_roles",
																		room_id: props.room.id,
																		user_id: i.user_id,
																		x: e.clientX,
																		y: e.clientY,
																	});
																}}
															>
																<em>edit roles...</em>
															</button>
														</li>
													</Show>
												</ul>
											</div>
										</div>
										<Show when={hasModerationFlags(i)}>
											<MemberFlags member={i} />
										</Show>
										<div style="flex:1"></div>
										<button
											type="button"
											class="button icon-button more-button"
											ref={moreTooltip.content}
											onClick={(e) => {
												e.stopPropagation();
												setMenu({
													type: "user",
													room_id: props.room.id,
													user_id: i.user_id,
													x: e.clientX,
													y: e.clientY,
													admin: false,
												});
											}}
										>
											<Icon src={icMore} alt="more..." />
										</button>
									</li>
								);
							}}
						</For>
					</ul>
					<div ref={setBottom}></div>
				</Show>
			</div>
			<Show when={selectedUserId()}>
				{(id) => (
					<Resizable
						storageKey="member-panel-width"
						initialWidth={360}
						minWidth={300}
						maxWidth={640}
					>
						<MemberPanel
							room={props.room}
							user_id={id()}
							onClose={() => setSelectedUserId(null)}
						/>
					</Resizable>
				)}
			</Show>
		</div>
	);
}
