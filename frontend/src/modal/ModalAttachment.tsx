import { createSignal } from "solid-js";
import { Checkbox } from "../icons";
import { Modal } from "./mod";
// import { useChannel } from "../channelctx";
// import { useApi } from "../api";

type ModalAttachmentProps = {
	channel_id: string;
	local_id: string;
};

export const ModalAttachment = (_props: ModalAttachmentProps) => {
	const [spoiler, setSpoiler] = createSignal(false);
	const [exif, setExif] = createSignal(false);

	// // NOTE: this is probably undefined, since i don't have a channel context
	// // how can i get the actual channel context here? do i need to pass it as a prop?
	// const [chan, updateChan] = useChannel()!;
	// const api = useApi();

	// updateChan("attachments", (atts) => {
	// 	return atts.map(att => {
	// 		if (att.local_id === props.local_id) {
	// 			// TODO: make media patch accept filename, spoiler, exif
	// 			api.client.http.PATCH("/api/v1/media/{media_id}", {
	// 				params: { path: { media_id: "00000000-0000-7000-0000-000000000000" } },
	// 				body: { alt: "alt text" }
	// 			});
	// 			return att;
	// 		} else {
	// 			return att;
	// 		}
	// 	})
	// })

	return (
		<Modal>
			<div style="width:300px">
				<h2>attachment</h2>
				{/* TODO: show attachment thumbnail (instead of this div) */}
				<div style="height:70px;width:100px;background:red;border-radius:4px;margin:8px 0">
				</div>
				<label style="display:block;margin:4px 0">
					<h3 class="dim">filename</h3>
					<input
						type="text"
						value="original-filename.ext"
						style="padding:4px"
					/>
				</label>
				<label style="display:block;margin:4px 0">
					<h3 class="dim">alt text</h3>
					<input
						type="text"
						placeholder="add a description"
						style="padding:4px"
					/>
				</label>
				<div class="option">
					<input
						id="opt-spoiler"
						type="checkbox"
						checked={spoiler()}
						onInput={(e) => setSpoiler(e.currentTarget.checked)}
						style="display:none"
					/>
					<Checkbox checked={spoiler()} />
					<label for="opt-spoiler">
						<div>Mark as spoiler</div>
						<div class="dim">
							Todo write something here idk
						</div>
					</label>
				</div>
				<div class="option">
					<input
						id="opt-exif"
						type="checkbox"
						checked={exif()}
						onInput={(e) => setExif(e.currentTarget.checked)}
						style="display:none"
					/>
					<Checkbox checked={exif()} />
					<label for="opt-exif">
						<div>Include metadata</div>
						<div class="dim">
							Todo write something here idk
						</div>
					</label>
				</div>
				<div class="bottom">
					<button>cancel</button>
					<button class="primary">save</button>
				</div>
			</div>
		</Modal>
	);
};
