import { onMount, type ParentProps } from "solid-js";
import { type Modal as ModalType, useCtx } from "../context.ts";
import { ModalResetPassword } from "../components/features/user_settings/mod.tsx";
import { ModalPalette } from "./ModalPalette.tsx";
import { ModalMessageEdits } from "./ModalMessageEdits.tsx";
import { ModalMedia } from "./ModalMedia.tsx";
import { ModalChannelCreate } from "./ModalChannelCreate";
import { ModalTagEditor } from "./ModalTagEditor.tsx";
import { ModalExportData } from "./ModalExportData.tsx";
import { useModals } from "../contexts/modal.tsx";
import { ModalReactions } from "./ModalReactions.tsx";
import { ModalNotifications } from "./ModalNotifications.tsx";
import { ModalPrivacy } from "./ModalPrivacy.tsx";
import { ModalAttachment } from "./ModalAttachment.tsx";
import { ModalInviteCreate } from "./ModalInviteCreate.tsx";
import { ModalChannelTopic } from "./ModalChannelTopic.tsx";
import { ModalLink } from "./ModalLink.tsx";
import { ModalKick } from "./ModalKick.tsx";
import { ModalBan } from "./ModalBan.tsx";
import { ModalTimeout } from "./ModalTimeout.tsx";
import { ModalCameraPreview } from "./ModalCameraPreview.tsx";
import { ModalRoomCreate } from "./ModalRoomCreate";
import { useApi2 } from "@/api";

export const Modal = (
	props: ParentProps & { onKeyDown?: (e: KeyboardEvent) => void },
) => {
	const [, modalCtl] = useModals();
	return (
		<div
			class="modal"
			onKeyDown={props.onKeyDown}
			tabindex="-1"
			autofocus
		>
			<div class="bg" onClick={() => modalCtl.close()}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					{props.children}
				</div>
			</div>
		</div>
	);
};

export type { ModalType };

// Type guard functions for modal type discrimination
function isAlert(
	modal: ModalType,
): modal is Extract<ModalType, { type: "alert" }> {
	return modal.type === "alert";
}

function isConfirm(
	modal: ModalType,
): modal is Extract<ModalType, { type: "confirm" }> {
	return modal.type === "confirm";
}

function isPrompt(
	modal: ModalType,
): modal is Extract<ModalType, { type: "prompt" }> {
	return modal.type === "prompt";
}

function isMedia(
	modal: ModalType,
): modal is Extract<ModalType, { type: "media" }> {
	return modal.type === "media";
}

function isMessageEdits(
	modal: ModalType,
): modal is Extract<ModalType, { type: "message_edits" }> {
	return modal.type === "message_edits";
}

function isResetPassword(
	modal: ModalType,
): modal is Extract<ModalType, { type: "reset_password" }> {
	return modal.type === "reset_password";
}

function isPalette(
	modal: ModalType,
): modal is Extract<ModalType, { type: "palette" }> {
	return modal.type === "palette";
}

function isChannelCreate(
	modal: ModalType,
): modal is Extract<ModalType, { type: "channel_create" }> {
	return modal.type === "channel_create";
}

function isTagEditor(
	modal: ModalType,
): modal is Extract<ModalType, { type: "tag_editor" }> {
	return modal.type === "tag_editor";
}

function isExportData(
	modal: ModalType,
): modal is Extract<ModalType, { type: "export_data" }> {
	return modal.type === "export_data";
}

function isViewReactions(
	modal: ModalType,
): modal is Extract<ModalType, { type: "view_reactions" }> {
	return modal.type === "view_reactions";
}

function isPrivacy(
	modal: ModalType,
): modal is Extract<ModalType, { type: "privacy" }> {
	return modal.type === "privacy";
}

function isNotifications(
	modal: ModalType,
): modal is Extract<ModalType, { type: "notifications" }> {
	return modal.type === "notifications";
}

function isAttachment(
	modal: ModalType,
): modal is Extract<ModalType, { type: "attachment" }> {
	return modal.type === "attachment";
}

function isInviteCreate(
	modal: ModalType,
): modal is Extract<ModalType, { type: "invite_create" }> {
	return modal.type === "invite_create";
}

function isChannelTopic(
	modal: ModalType,
): modal is Extract<ModalType, { type: "channel_topic" }> {
	return modal.type === "channel_topic";
}

function isLink(
	modal: ModalType,
): modal is Extract<ModalType, { type: "link" }> {
	return modal.type === "link";
}

function isKick(
	modal: ModalType,
): modal is Extract<ModalType, { type: "kick" }> {
	return modal.type === "kick";
}

function isBan(modal: ModalType): modal is Extract<ModalType, { type: "ban" }> {
	return modal.type === "ban";
}

function isTimeout(
	modal: ModalType,
): modal is Extract<ModalType, { type: "timeout" }> {
	return modal.type === "timeout";
}

function isCameraPreview(
	modal: ModalType,
): modal is Extract<ModalType, { type: "camera_preview" }> {
	return modal.type === "camera_preview";
}

function isRoomCreate(
	modal: ModalType,
): modal is Extract<ModalType, { type: "room_create" }> {
	return modal.type === "room_create";
}

export function getModal(modal: ModalType) {
	const api2 = useApi2();
	if (isAlert(modal)) {
		return <ModalAlert text={modal.text} />;
	}
	if (isConfirm(modal)) {
		return <ModalConfirm text={modal.text} cont={modal.cont} />;
	}
	if (isPrompt(modal)) {
		return <ModalPrompt text={modal.text} cont={modal.cont} />;
	}
	if (isMedia(modal)) {
		return <ModalMedia media={modal.media} />;
	}
	if (isMessageEdits(modal)) {
		return (
			<ModalMessageEdits
				thread_id={modal.channel_id}
				message_id={modal.message_id}
			/>
		);
	}
	if (isResetPassword(modal)) {
		return <ModalResetPassword />;
	}
	if (isPalette(modal)) {
		return <ModalPalette />;
	}
	if (isChannelCreate(modal)) {
		return (
			<ModalChannelCreate
				room_id={modal.room_id}
				cont={modal.cont}
			/>
		);
	}
	if (isTagEditor(modal)) {
		return (
			<ModalTagEditor
				forumChannelId={modal.forumChannelId}
				tag={modal.tag}
				onSave={modal.onSave}
				onClose={modal.onClose}
			/>
		);
	}
	if (isExportData(modal)) {
		return <ModalExportData />;
	}
	if (isViewReactions(modal)) {
		return (
			<ModalReactions
				channel_id={modal.channel_id}
				message_id={modal.message_id}
			/>
		);
	}
	if (isPrivacy(modal)) {
		return <ModalPrivacy room_id={modal.room_id} />;
	}
	if (isNotifications(modal)) {
		return <ModalNotifications room_id={modal.room_id} />;
	}
	if (isAttachment(modal)) {
		return (
			<ModalAttachment
				channel_id={modal.channel_id}
				local_id={modal.local_id}
			/>
		);
	}
	if (isInviteCreate(modal)) {
		return (
			<ModalInviteCreate
				channel_id={modal.channel_id}
				room_id={modal.room_id}
			/>
		);
	}
	if (isChannelTopic(modal)) {
		return (
			<ModalChannelTopic
				channel_id={modal.channel_id}
			/>
		);
	}
	if (isLink(modal)) {
		return (
			<ModalLink
				editor={modal.editor}
			/>
		);
	}
	if (isKick(modal)) {
		return (
			<ModalKick
				api={api2}
				room_id={modal.room_id}
				user_id={modal.user_id}
			/>
		);
	}
	if (isBan(modal)) {
		return (
			<ModalBan
				api={api2}
				room_id={modal.room_id}
				user_id={modal.user_id}
			/>
		);
	}
	if (isTimeout(modal)) {
		return (
			<ModalTimeout
				api={api2}
				room_id={modal.room_id}
				user_id={modal.user_id}
			/>
		);
	}
	if (isCameraPreview(modal)) {
		return (
			<ModalCameraPreview
				stream={modal.stream}
			/>
		);
	}
	if (isRoomCreate(modal)) {
		return (
			<ModalRoomCreate
				cont={modal.cont}
			/>
		);
	}
}

const ModalAlert = (props: { text: string }) => {
	const [, modalCtl] = useModals();
	let btn: HTMLButtonElement | undefined;
	onMount(() => btn?.focus());
	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Escape") {
					modalCtl.close();
				}
			}}
		>
			<p>{props.text}</p>
			<div class="bottom">
				<button ref={btn} onClick={modalCtl.close}>
					okay!
				</button>
			</div>
		</Modal>
	);
};

const ModalConfirm = (
	props: { text: string; cont: (bool: boolean) => void },
) => {
	const [, modalCtl] = useModals();
	let cancelBtn: HTMLButtonElement | undefined;
	onMount(() => cancelBtn?.focus());
	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Enter") {
					props.cont(true);
					modalCtl.close();
				} else if (e.key === "Escape") {
					props.cont(false);
					modalCtl.close();
				}
			}}
		>
			<p>{props.text}</p>
			<div class="bottom">
				<button
					onClick={() => {
						props.cont(true);
						modalCtl.close();
					}}
				>
					okay!
				</button>
				<button
					ref={cancelBtn}
					onClick={() => {
						props.cont(false);
						modalCtl.close();
					}}
				>
					nevermind...
				</button>
			</div>
		</Modal>
	);
};

const ModalPrompt = (
	props: { text: string; cont: (s: string | null) => void },
) => {
	const [, modalCtl] = useModals();
	let input: HTMLInputElement | undefined;
	onMount(() => input?.focus());
	return (
		<Modal>
			<p>{props.text}</p>
			<div style="height: 8px"></div>
			<form
				onSubmit={(e) => {
					e.preventDefault();
					const form = e.target as HTMLFormElement;
					const input = form.elements.namedItem(
						"text",
					) as HTMLInputElement;
					props.cont(input.value);
					modalCtl.close();
				}}
			>
				<input ref={input} type="text" name="text" />
				<div class="bottom">
					<input type="submit" value="done!"></input>{" "}
					<button
						onClick={() => {
							props.cont(null);
							modalCtl.close();
						}}
					>
						nevermind...
					</button>
				</div>
			</form>
		</Modal>
	);
};
