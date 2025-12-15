import { type ParentProps } from "solid-js";
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

export const Modal = (props: ParentProps) => {
	const [, modalCtl] = useModals();
	return (
		<div class="modal">
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
	switch (modal.type) {
		case "alert": {
			return <ModalAlert text={modal.text} />;
		}
		case "confirm": {
			return <ModalConfirm text={modal.text} cont={modal.cont} />;
		}
		case "prompt": {
			return <ModalPrompt text={modal.text} cont={modal.cont} />;
		}
		case "media": {
			return <ModalMedia media={modal.media} />;
		}
		case "message_edits": {
			return (
				<ModalMessageEdits
					thread_id={modal.channel_id}
					message_id={modal.message_id}
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
					room_id={modal.room_id}
					cont={modal.cont}
				/>
			);
		}
		case "tag_editor": {
			return (
				<ModalTagEditor
					forumChannelId={modal.forumChannelId}
					tag={modal.tag}
					onSave={modal.onSave}
					onClose={modal.onClose}
				/>
			);
		}
		case "export_data": {
			return <ModalExportData />;
		}
		case "view_reactions": {
			return (
				<ModalReactions
					channel_id={modal.channel_id}
					message_id={modal.message_id}
				/>
			);
		}
		case "privacy": {
			return <ModalPrivacy channel_id={modal.room_id} />;
		}
		case "notifications": {
			return <ModalNotifications channel_id={modal.room_id} />;
		}
	}
}

const ModalAlert = (props: { text: string }) => {
	const [, modalCtl] = useModals();
	return (
		<Modal>
			<p>{props.text}</p>
			<div class="bottom">
				<button onClick={modalCtl.close}>
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
	return (
		<Modal>
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
				<input type="text" name="text" autofocus />
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
