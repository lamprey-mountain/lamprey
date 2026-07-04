import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { Icon } from "@/atoms/Icon";
import { useOptionalChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { useMessageToolbar } from "./message-toolbar-context.tsx";
import { MessageT } from "@/types";
import { icEdit, icMore, icReactionAdd, icReply } from "@/utils/icons";
import { autoUpdate, offset, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { ReactionKey } from "ts-sdk";

const areReactionKeysEqual = (a: ReactionKey, b: ReactionKey): boolean => {
	if (a.type !== b.type) return false;
	if (a.type === "Text" && b.type === "Text") return a.content === b.content;
	if (a.type === "Custom" && b.type === "Custom") return a.id === b.id;
	return false;
};

export const MessageToolbar = (props: { message: MessageT }) => {
	const api2 = useApi();
	const ctx = useCtx();
	const { setMenu } = useMenu();
	const { setLocked } = useMessageToolbar();
	const [ch, chUpdate] = useOptionalChannel();
	let reactionButtonRef: HTMLButtonElement | undefined;

	const isReactionPickerOpen = () => {
		const p = ctx.popout();
		return p && p.id === "emoji" && p.ref === reactionButtonRef;
		// return p && p.id === "emoji";
	};

	const addReaction = (emoji: string) => {
		const existing = props.message.reactions?.find((r) =>
			areReactionKeysEqual(r.key, { type: "Text", content: emoji }),
		);
		if (!existing || !existing.self) {
			api2.reactions.add(
				props.message.channel_id,
				props.message.id,
				`t:${emoji}`,
			);
		}
	};

	const closeReactionPicker = (e: MouseEvent) => {
		const popoutEl = document.querySelector(".popout");

		if (
			reactionButtonRef &&
			!reactionButtonRef.contains(e.target as Node) &&
			(!popoutEl || !popoutEl.contains(e.target as Node))
		) {
			setLocked(false);
			ctx.setPopout(null);
		}
	};

	document.addEventListener("click", closeReactionPicker);
	onCleanup(() => document.removeEventListener("click", closeReactionPicker));

	const currentUser = useCurrentUser();
	const isOwnMessage = () => {
		return currentUser()?.id === props.message.author_id;
	};

	const canEditMessage = () => {
		return (
			props.message.latest_version.type === "DefaultMarkdown" &&
			!props.message.is_local &&
			isOwnMessage()
		);
	};

	const handleAddReaction = (e: MouseEvent) => {
		e.stopPropagation();
		if (isReactionPickerOpen()) {
			setLocked(false);
			ctx.setPopout(null);
		} else {
			setLocked(true);
			ctx.setPopout({
				id: "emoji",
				ref: reactionButtonRef,
				placement: "left-start",
				props: {
					selected: (emoji: string | null, keepOpen: boolean) => {
						if (emoji) {
							addReaction(emoji);
						}
						if (!keepOpen) {
							setLocked(false);
							ctx.setPopout(null);
						}
					},
				},
			});
		}
	};

	const handleReply = () => {
		if (!ch || !chUpdate) return;
		chUpdate("reply_id", props.message.id);
	};

	const handleEdit = () => {
		if (!canEditMessage() || !chUpdate) return;
		chUpdate("editingMessage", {
			message_id: props.message.id,
			selection: "end",
		});
	};

	const handleContextMenu = (e: MouseEvent) => {
		e.preventDefault();

		const button = e.currentTarget as HTMLButtonElement;
		const rect = button.getBoundingClientRect();

		queueMicrotask(() => {
			setMenu({
				x: rect.left,
				y: rect.bottom,
				type: "message",
				channel_id: props.message.channel_id,
				message_id: props.message.id,
				version_id: props.message.latest_version.version_id,
			});
		});
	};

	return (
		<div class="message-toolbar">
			<button
				type="button"
				class="button"
				ref={reactionButtonRef}
				onClick={handleAddReaction}
				title="Add reaction"
				aria-label="Add reaction"
			>
				<Icon src={icReactionAdd} />
			</button>
			<button
				type="button"
				class="button"
				onClick={handleReply}
				title="Reply"
				aria-label="Reply"
			>
				<Icon src={icReply} />
			</button>
			<Show when={canEditMessage()}>
				<button
					type="button"
					class="button"
					onClick={handleEdit}
					title="Edit"
					aria-label="Edit"
				>
					<Icon src={icEdit} />
				</button>
			</Show>
			<button
				type="button"
				class="button"
				onClick={handleContextMenu}
				title="More options"
				aria-label="More options"
			>
				<Icon src={icMore} />
			</button>
		</div>
	);
};

export const MessageToolbarMount = () => {
	const { target, setTarget, setContainerRef } = useMessageToolbar();
	const [tipEl, setTipEl] = createSignal<HTMLDivElement>();

	const pos = useFloating(() => target()?.element ?? null, tipEl, {
		whileElementsMounted: autoUpdate,
		strategy: "absolute",
		placement: "top-end",
		middleware: [shift({ padding: 8 }), offset({ mainAxis: -8 })],
	});

	const handleClick = (e: MouseEvent) => {
		if (e.target && tipEl()?.contains(e.target as HTMLElement)) return;
		if (target()) setTarget(null);
	};

	createEffect(() => {
		if (target()) document.addEventListener("click", handleClick);
		onCleanup(() => document.removeEventListener("click", handleClick));
	});

	return (
		<Show when={target()}>
			{(t) => (
				<div
					ref={(el) => {
						setTipEl(el);
						setContainerRef(el);
					}}
					style={{
						position: pos.strategy,
						top: `${pos.y ?? 0}px`,
						left: `${pos.x ?? 0}px`,
						"z-index": 1000,
					}}
					onMouseLeave={(e) => {
						const related = e.relatedTarget;
						const msgEl = t().element;
						// don't clear if moving back onto the message that owns this toolbar
						if (related instanceof Node && msgEl?.contains(related)) return;
						setTarget(null);
					}}
				>
					<MessageToolbar message={t().message} />
				</div>
			)}
		</Show>
	);
};
