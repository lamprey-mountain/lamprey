import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	type ParentProps,
	Show,
} from "solid-js";
import { createStore } from "solid-js/store";
import {
	autoUpdate,
	type ClientRectObject,
	computePosition,
	flip,
	offset,
	type ReferenceElement,
	shift,
} from "@floating-ui/dom";
import { Portal } from "solid-js/web";
import { useCtx } from "../context.ts";
import type { Menu } from "./menu.tsx";
import {
	useAutocomplete,
	useFormattingToolbar,
	useMenu,
	useUserPopout,
} from "./mod.tsx";
import { FormattingToolbar } from "./FormattingToolbar.tsx";
import { useApi } from "../api.tsx";
import {
	ChannelMenu,
	FolderMenu,
	MessageMenu,
	RoomMenu,
	UserMenu,
} from "../menus/mod.ts";
import { EmojiPicker } from "../EmojiPicker.tsx";
import { UserView } from "../User.tsx";
import { ThreadPopout } from "../components/features/chat/ThreadPopout.tsx";
import { Autocomplete } from "../Autocomplete.tsx";
import { PopupEventEditor, useCalendarPopup } from "../Calendar.tsx";
import { getModal } from "../modals/mod.tsx";
import { Modal, useModals } from "./modal.tsx";

export function OverlayProvider(props: ParentProps) {
	const ctx = useCtx();
	const { menu } = useMenu();
	const { state: autocompleteState } = useAutocomplete();
	const { userView } = useUserPopout();
	const { toolbar, hideToolbar } = useFormattingToolbar();
	const api = useApi();
	const [modals] = useModals();
	const { popup: calendarPopup, closePopup: closeCalendarPopup } =
		useCalendarPopup();

	const [toolbarRef, setToolbarRef] = createSignal<HTMLElement>();
	const [toolbarFloating, setToolbarFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "fixed" as const,
	});

	const [popupRef, setPopupRef] = createSignal<HTMLElement>();
	const [popupFloating, setPopupFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = toolbar().reference;
		const floating = toolbarRef();
		if (!reference || !floating) return;

		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					placement: "top",
					middleware: [
						offset({ mainAxis: 8 }),
						flip(),
						shift({ padding: 8 }),
					],
				}).then(({ x, y, strategy }) => {
					setToolbarFloating({ x, y, strategy: strategy as any });
				});
			},
		);
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
				setPopupFloating({ x, y, strategy: strategy as any });
			});
		});
		onCleanup(cleanup);
	});

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [menuFloating, setMenuFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = menuParentRef();
		const floating = menuRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: "right-start",
				}).then(({ x, y, strategy }) => {
					setMenuFloating({ x, y, strategy: strategy as any });
				});
			},
		);
		onCleanup(cleanup);
	});

	const [autocompleteRef, setAutocompleteRef] = createSignal<HTMLElement>();
	const [autocompleteFloating, setAutocompleteFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = autocompleteState.reference;
		const floating = autocompleteRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [offset({ mainAxis: 8 })],
					placement: "top-start",
				}).then(({ x, y, strategy }) => {
					setAutocompleteFloating({ x, y, strategy: strategy as any });
				});
			},
		);
		onCleanup(cleanup);
	});

	const [userViewRef, setUserViewRef] = createSignal<HTMLElement>();
	const [userViewFloating, setUserViewFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = userView()?.ref;
		const floating = userViewRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: userView()?.source === "message"
						? "right-start"
						: "left-start",
				}).then(({ x, y, strategy }) => {
					setUserViewFloating({ x, y, strategy: strategy as any });
				});
			},
		);
		onCleanup(cleanup);
	});

	const [threadsViewRef, setThreadsViewRef] = createSignal<HTMLElement>();
	const [threadsViewFloating, setThreadsViewFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = ctx.threadsView()?.ref;
		const floating = threadsViewRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: "bottom-end",
				}).then(({ x, y, strategy }) => {
					setThreadsViewFloating({ x, y, strategy: strategy as any });
				});
			},
		);
		onCleanup(cleanup);
	});

	const [popoutRef, setPopoutRef] = createSignal<HTMLElement>();
	const [popoutFloating, setPopoutFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = ctx.popout()?.ref;
		const floating = popoutRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: ctx.popout()?.placement ?? "top",
				}).then(({ x, y, strategy }) => {
					setPopoutFloating({ x, y, strategy: strategy as any });
				});
			},
		);
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
		}
	}

	const userViewData = createMemo(() => {
		const uv = userView();
		if (!uv) return null;
		const user = api.users.fetch(() => uv.user_id);
		const room_member = uv.room_id
			? api.room_members.fetch(() => uv.room_id!, () => uv.user_id)
			: () => null;
		const thread_member = uv.channel_id
			? api.thread_members.fetch(() => uv.channel_id!, () => uv.user_id)
			: () => null;
		return { user, room_member, thread_member };
	});

	return (
		<>
			{props.children}
			<Portal mount={document.getElementById("overlay")!}>
				<For each={modals}>{(modal) => getModal(modal as any)}</For>
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
						<EmojiPicker {...ctx.popout()?.props} />
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
						<UserView
							user={userViewData()!.user()!}
							room_member={userViewData()!.room_member() ?? undefined}
							thread_member={userViewData()!.thread_member() ?? undefined}
						/>
					</div>
				</Show>
				<Show when={ctx.threadsView()}>
					<div
						ref={setThreadsViewRef}
						style={{
							position: threadsViewFloating.strategy,
							top: "0px",
							left: "0px",
							translate:
								`${threadsViewFloating.x}px ${threadsViewFloating.y}px`,
							"z-index": 100,
						}}
					>
						<ThreadPopout channel_id={ctx.threadsView()!.channel_id} />
					</div>
				</Show>
				<Show when={autocompleteState.visible}>
					<div
						ref={setAutocompleteRef}
						style={{
							position: autocompleteFloating.strategy,
							top: "0px",
							left: "0px",
							translate:
								`${autocompleteFloating.x}px ${autocompleteFloating.y}px`,
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
						<PopupEventEditor
							channel_id={calendarPopup()!.props.channel_id}
							event={calendarPopup()!.props.event}
							onClose={closeCalendarPopup}
						/>
					</div>
				</Show>
			</Portal>
		</>
	);
}
