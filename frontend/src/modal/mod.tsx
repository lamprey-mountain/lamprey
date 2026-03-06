import { onMount, type ParentProps } from "solid-js";
import { type Modal as ContextModal, useCtx } from "../context.ts";
import { ModalResetPassword } from "../user_settings/mod.tsx";
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
import { useApi } from "../api";

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

export function getModal(modal: ContextModal) {
	const api = useApi();
	switch ((modal as any).type) {
		case "alert": {
			return <ModalAlert text={(modal as any).text} />;
		}
		case "confirm": {
			return (
				<ModalConfirm text={(modal as any).text} cont={(modal as any).cont} />
			);
		}
		case "prompt": {
			return (
				<ModalPrompt text={(modal as any).text} cont={(modal as any).cont} />
			);
		}
		case "media": {
			return <ModalMedia media={(modal as any).media} />;
		}
		case "message_edits": {
			return (
				<ModalMessageEdits
					thread_id={(modal as any).channel_id}
					message_id={(modal as any).message_id}
				/>
			);
		}
		case "reset_password": {
			return <ModalResetPassword />;
		}
		case "palette": {
			return <ModalPalette />;
		}
		case "channel_create": {
			return (
				<ModalChannelCreate
					room_id={(modal as any).room_id}
					cont={(modal as any).cont}
				/>
			);
		}
		case "tag_editor": {
			return (
				<ModalTagEditor
					forumChannelId={(modal as any).forumChannelId}
					tag={(modal as any).tag}
					onSave={(modal as any).onSave}
					onClose={(modal as any).onClose}
				/>
			);
		}
		case "export_data": {
			return <ModalExportData />;
		}
		case "view_reactions": {
			return (
				<ModalReactions
					channel_id={(modal as any).channel_id}
					message_id={(modal as any).message_id}
				/>
			);
		}
		case "privacy": {
			return <ModalPrivacy room_id={(modal as any).room_id} />;
		}
		case "notifications": {
			return <ModalNotifications room_id={(modal as any).room_id} />;
		}
		case "attachment": {
			return (
				<ModalAttachment
					channel_id={(modal as any).channel_id}
					local_id={(modal as any).local_id}
				/>
			);
		}
		case "invite_create": {
			return (
				<ModalInviteCreate
					channel_id={(modal as any).channel_id}
					room_id={(modal as any).room_id}
				/>
			);
		}
		case "channel_topic": {
			return (
				<ModalChannelTopic
					channel_id={(modal as any).channel_id}
				/>
			);
		}
		case "link": {
			return (
				<ModalLink
					editor={(modal as any).editor}
				/>
			);
		}
		case "kick": {
			return (
				<ModalKick
					api={api}
					room_id={(modal as any).room_id}
					user_id={(modal as any).user_id}
				/>
			);
		}
		case "ban": {
			return (
				<ModalBan
					api={api}
					room_id={(modal as any).room_id}
					user_id={(modal as any).user_id}
				/>
			);
		}
		case "timeout": {
			return (
				<ModalTimeout
					api={api}
					room_id={(modal as any).room_id}
					user_id={(modal as any).user_id}
				/>
			);
		}
		case "camera_preview": {
			return (
				<ModalCameraPreview
					stream={(modal as any).stream}
				/>
			);
		}
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
