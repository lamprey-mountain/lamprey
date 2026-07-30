import { createSignal } from "solid-js";
import type { Media } from "ts-sdk";
import { type Api, useApi } from "@/api";
import { useModals } from "@/contexts/modal";
import { getThumb } from "@/media/util";
import { Modal } from "./mod";

interface ModalEmojiUploadProps {
	room_id: string;
	media: Media;
}

export const ModalEmojiUpload = (props: ModalEmojiUploadProps) => {
	const api = useApi();
	const [, modalCtl] = useModals();

	const [name, setName] = createSignal("unknown");
	const [loading, setLoading] = createSignal(false);

	const save = () => {
		setLoading(true);
		api.client.http
			.POST("/api/v1/room/{room_id}/emoji", {
				params: {
					path: {
						room_id: props.room_id,
					},
				},
				body: { animated: false, media_id: props.media.id, name: name() },
			})
			.then(() => {
				modalCtl.close();
			});
	};

	return (
		<Modal>
			<div class="modal-emoji-upload">
				<h3>upload emoji</h3>

				<div class="main">
					<img
						class="emoji"
						src={getThumb(props.media)}
						alt={`:${name()}:`}
						title={`:${name()}:`}
					/>

					<div>
						<form
							onSubmit={(e) => {
								e.preventDefault();
								save();
							}}
						>
							<label>
								<h3 class="dim">name</h3>

								<input
									type="text"
									placeholder="monkaw"
									ref={(el) => queueMicrotask(() => el.focus())}
									onInput={(e) => setName(e.target.value)}
								/>
							</label>
						</form>
					</div>
				</div>

				<div class="bottom">
					<button
						type="button"
						class="button link"
						onClick={() => modalCtl.close()}
						disabled={loading()}
					>
						cancel
					</button>
					<button
						type="button"
						class="button primary"
						onClick={save}
						disabled={loading()}
					>
						{loading() ? "save..." : "save"}
					</button>
				</div>
			</div>
		</Modal>
	);
};
