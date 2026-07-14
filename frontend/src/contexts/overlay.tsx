// TODO: split apart this component?

import {
	autoUpdate,
	type ClientRectObject,
	computePosition,
	flip,
	offset,
	type ReferenceElement,
	type Strategy,
	shift,
} from "@floating-ui/dom";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	type ParentProps,
	Show,
	Switch,
	Match,
} from "solid-js";
import { createStore } from "solid-js/store";
import { Portal } from "solid-js/web";
import { useRoomMembers, useThreadMembers, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import { Autocomplete } from "@/atoms/Autocomplete.tsx";
import { EmojiPicker } from "@/atoms/EmojiPicker.tsx";
import { ThreadPopout } from "@/components/features/chat/ThreadPopout.tsx";
import {
	PopupEventEditor,
	useCalendarPopup,
} from "@/components/shared/Calendar";
import { UserProfileEdit } from "@/components/shared/UserProfileEdit.tsx";
import { UserProfile } from "@/components/shared/UserProfile.tsx";
import {
	ChannelMenu,
	FolderMenu,
	MessageMenu,
	PermissionOverwriteMenu,
	RoomMenu,
	TopicMenu,
	UserMenu,
} from "@/menus/mod.ts";
import { getModal } from "@/modals/mod.tsx";
import { FormattingToolbar } from "./FormattingToolbar.tsx";
import type { Menu } from "./menu.tsx";
import {
	useAutocomplete,
	useFormattingToolbar,
	useMenu,
	useUserPopout,
} from "./mod.tsx";
import { type Modal, useModals } from "./modal.tsx";

type FloatingPosition = { x: number; y: number; strategy: Strategy };

export function OverlayProvider(props: ParentProps) {
	const ctx = useCtx();
	const { menu } = useMenu();
	const { state: autocompleteState } = useAutocomplete();
	const { userView } = useUserPopout();
	const { toolbar, hideToolbar } = useFormattingToolbar();
	const users2 = useUsers();
	const roomMembers2 = useRoomMembers();
	const threadMembers2 = useThreadMembers();
	const [modals] = useModals();
	const { popup: calendarPopup, closePopup: closeCalendarPopup } =
		useCalendarPopup();

	const [toolbarRef, setToolbarRef] = createSignal<HTMLElement>();
	const [toolbarFloating, setToolbarFloating] = createStore<FloatingPosition>({
		x: 0,
		y: 0,
		strategy: "fixed",
	});

	const [popupRef, setPopupRef] = createSignal<HTMLElement>();
	const [popupFloating, setPopupFloating] = createStore<FloatingPosition>({
		x: 0,
		y: 0,
		strategy: "absolute",
	});

	createEffect(() => {
		const reference = toolbar().reference;
		const floating = toolbarRef();
		if (!reference || !floating) return;

		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				placement: "top",
				middleware: [offset({ mainAxis: 8 }), flip(), shift({ padding: 8 })],
			}).then(({ x, y, strategy }) => {
				setToolbarFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	createEffect(() => {
		const referenceEl = calendarPopup()?.ref;
		const floatingEl = popupRef();
		if (!referenceEl || !floatingEl) return;

		const cleanup = autoUpdate(referenceEl, floatingEl, () => {
			computePosition(referenceEl, floatingEl, {
				placement: calendarPopup()?.placement ?? "bottom-start",
				middleware: [
					offset({ mainAxis: 8 }),
					flip(),
					shift({ mainAxis: true, crossAxis: true, padding: 8 }),
				],
			}).then(({ x, y, strategy }) => {
				setPopupFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [menuFloating, setMenuFloating] = createStore<FloatingPosition>({
		x: 0,
		y: 0,
		strategy: "absolute",
	});

	createEffect(() => {
		const reference = menuParentRef();
		const floating = menuRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
				placement: "right-start",
			}).then(({ x, y, strategy }) => {
				setMenuFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const [autocompleteRef, setAutocompleteRef] = createSignal<HTMLElement>();
	const [autocompleteFloating, setAutocompleteFloating] =
		createStore<FloatingPosition>({
			x: 0,
			y: 0,
			strategy: "absolute",
		});

	createEffect(() => {
		const reference = autocompleteState.reference;
		const floating = autocompleteRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				middleware: [offset({ mainAxis: 8 })],
				placement: "top-start",
			}).then(({ x, y, strategy }) => {
				setAutocompleteFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const [userViewRef, setUserViewRef] = createSignal<HTMLElement>();
	const [userViewFloating, setUserViewFloating] = createStore<FloatingPosition>(
		{
			x: 0,
			y: 0,
			strategy: "absolute",
		},
	);

	createEffect(() => {
		const reference = userView()?.ref;
		const floating = userViewRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			const v = userView();
			computePosition(reference, floating, {
				middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
				placement:
					v?.source === "message"
						? "right-start"
						: v?.source === "user-tray"
							? "top-start"
							: "left-start",
			}).then(({ x, y, strategy }) => {
				setUserViewFloating({ x, y, strategy });
				if (v?.source === "user-tray") {
					floating.style.width = `${reference.getBoundingClientRect().width - 16}px`;
				} else {
					floating.style.width = "";
				}
			});
		});
		onCleanup(cleanup);
	});

	const [threadsViewRef, setThreadsViewRef] = createSignal<HTMLElement>();
	const [threadsViewFloating, setThreadsViewFloating] =
		createStore<FloatingPosition>({
			x: 0,
			y: 0,
			strategy: "absolute",
		});

	createEffect(() => {
		const reference = ctx.threadsView()?.ref;
		const floating = threadsViewRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
				placement: "bottom-end",
			}).then(({ x, y, strategy }) => {
				setThreadsViewFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const [popoutRef, setPopoutRef] = createSignal<HTMLElement>();
	const [popoutFloating, setPopoutFloating] = createStore<FloatingPosition>({
		x: 0,
		y: 0,
		strategy: "absolute",
	});

	createEffect(() => {
		const reference = ctx.popout()?.ref;
		const floating = popoutRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
				placement: ctx.popout()?.placement ?? "top",
			}).then(({ x, y, strategy }) => {
				setPopoutFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	createEffect(() => {
		menu();

		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				const m = menu();
				if (!m) return {} as ClientRectObject;
				return {
					x: m.x,
					y: m.y,
					left: m.x,
					top: m.y,
					right: m.x,
					bottom: m.y,
					width: 0,
					height: 0,
				};
			},
		});
	});

	// TODO: use Switch/Match instead?
	function getMenu(menu: Menu) {
		switch (menu.type) {
			case "room": {
				return <RoomMenu room_id={menu.room_id} />;
			}
			case "channel": {
				return <ChannelMenu channel_id={menu.channel_id} />;
			}
			case "message": {
				return (
					<MessageMenu
						channel_id={menu.channel_id}
						message_id={menu.message_id}
						version_id={menu.version_id}
					/>
				);
			}
			case "user": {
				return (
					<UserMenu
						user_id={menu.user_id}
						room_id={menu.room_id}
						channel_id={menu.channel_id}
						admin={menu.admin}
					/>
				);
			}
			case "folder": {
				return <FolderMenu folder_id={menu.folder_id} />;
			}
			case "topic": {
				return <TopicMenu channel_id={menu.channel_id} />;
			}
			case "permission_overwrite": {
				return (
					<PermissionOverwriteMenu
						channel_id={menu.channel_id}
						overwrite_id={menu.overwrite_id}
						overwrite_type={menu.overwrite_type}
						onDelete={menu.onDelete}
					/>
				);
			}
		}
	}

	const userViewData = createMemo(() => {
		const uv = userView();
		if (!uv) return null;
		const user = users2.use(() => uv.user_id);
		const room_member = uv.room_id
			? roomMembers2.use(() => `${uv.room_id!}:${uv.user_id}`)
			: () => null;
		const thread_member = uv.channel_id
			? threadMembers2.use(() => `${uv.channel_id!}:${uv.user_id}`)
			: () => null;
		return { user, room_member, thread_member };
	});

	return (
		<>
			{props.children}
			<Portal mount={document.getElementById("overlay")!}>
				<For each={modals}>{(modal) => getModal(modal as Modal)}</For>
				<Show when={menu()}>
					<div class="contextmenu">
						<div
							ref={setMenuRef}
							class="inner"
							style={{
								position: menuFloating.strategy,
								top: "0px",
								left: "0px",
								translate: `${menuFloating.x}px ${menuFloating.y}px`,
							}}
						>
							{getMenu(menu()!)}
						</div>
					</div>
				</Show>
				<Show when={ctx.popout()?.id === "emoji" && ctx.popout()?.ref}>
					<div
						ref={setPopoutRef}
						style={{
							position: popoutFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${popoutFloating.x}px ${popoutFloating.y}px`,
							"z-index": 100,
						}}
					>
						<EmojiPicker
							{...(ctx.popout()?.props as {
								selected: (value: string | null, shiftKey: boolean) => void;
							})}
						/>
					</div>
				</Show>
				<Show when={userViewData()?.user()}>
					<div
						ref={setUserViewRef}
						style={{
							position: userViewFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${userViewFloating.x}px ${userViewFloating.y}px`,
							"z-index": 100,
						}}
					>
						<Switch>
							<Match when={userView()?.source === "user-tray"}>
								<UserProfileEdit
									user={userViewData()?.user()!}
									room_member={userViewData()?.room_member() ?? undefined}
									thread_member={userViewData()?.thread_member() ?? undefined}
								/>
							</Match>
							<Match when={true}>
								<UserProfile
									user={userViewData()?.user()!}
									room_member={userViewData()?.room_member() ?? undefined}
									thread_member={userViewData()?.thread_member() ?? undefined}
								/>
							</Match>
						</Switch>
					</div>
				</Show>
				<Show when={ctx.threadsView()}>
					<div
						ref={setThreadsViewRef}
						style={{
							position: threadsViewFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${threadsViewFloating.x}px ${threadsViewFloating.y}px`,
							"z-index": 100,
						}}
					>
						<Show when={ctx.threadsView()?.channel_id}>
							{(cid) => <ThreadPopout channel_id={cid()} />}
						</Show>
					</div>
				</Show>
				<Show when={autocompleteState.visible}>
					<div
						ref={setAutocompleteRef}
						style={{
							position: autocompleteFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${autocompleteFloating.x}px ${autocompleteFloating.y}px`,
							"z-index": 100,
						}}
					>
						<Autocomplete />
					</div>
				</Show>
				<Show when={toolbar().visible}>
					<div
						ref={setToolbarRef}
						style={{
							position: toolbarFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${toolbarFloating.x}px ${toolbarFloating.y}px`,
							"z-index": 1000,
						}}
					>
						<FormattingToolbar onClose={hideToolbar} />
					</div>
				</Show>
				<Show when={calendarPopup()}>
					<div
						ref={setPopupRef}
						style={{
							position: popupFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${popupFloating.x}px ${popupFloating.y}px`,
							"z-index": 100,
						}}
					>
						<Show when={calendarPopup()?.props.channel_id}>
							{(cid) => (
								<PopupEventEditor
									channel_id={cid()}
									event={calendarPopup()?.props.event}
									onClose={closeCalendarPopup}
								/>
							)}
						</Show>
					</div>
				</Show>
			</Portal>
		</>
	);
}
