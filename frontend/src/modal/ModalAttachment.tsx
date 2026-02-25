import { createSignal, onMount, Show } from "solid-js";
import { Checkbox } from "../icons";
import { Modal } from "./mod";
import { useApi } from "../api";
import { useModals } from "../contexts/modal";
import { useCtx } from "../context";
import { getThumbFromId } from "../media/util";

type ModalAttachmentProps = {
	channel_id: string;
	local_id: string;
};

export const ModalAttachment = (props: ModalAttachmentProps) => {
	const api = useApi();
	const [, modalCtl] = useModals();
	const ctx = useCtx();
	const [filename, setFilename] = createSignal("");
	const [alt, setAlt] = createSignal("");
	const [spoiler, setSpoiler] = createSignal(false);
	const [exif, setExif] = createSignal(false);

	const channelCtx = ctx.channel_contexts.get(props.channel_id);
	const attachment = () => {
		if (!channelCtx) return null;
		const [ch] = channelCtx;
		return ch.attachments.find((a) => a.local_id === props.local_id);
	};

	onMount(() => {
		const att = attachment();
		if (att) {
			if (att.status === "uploaded") {
				setFilename(att.media.filename);
				setAlt(att.media.alt ?? "");
				setSpoiler(att.spoiler ?? false);
			} else {
				setFilename(att.filename ?? att.file.name);
				setAlt(att.alt ?? "");
				setSpoiler(att.spoiler ?? false);
			}
		}
	});

	const save = () => {
		const att = attachment();
		if (!att) return;

		if (att.status === "uploaded") {
			api.client.http.PATCH("/api/v1/media/{media_id}", {
				params: { path: { media_id: att.media.id } },
				body: {
					alt: alt() || null,
					filename: filename() || null,
				},
			});
		}

		const [ch, chUpdate] = channelCtx!;
		chUpdate(
			"attachments",
			ch.attachments.map((a) => {
				if (a.local_id === props.local_id) {
					if (a.status === "uploaded") {
						return {
							...a,
							spoiler: spoiler(),
							media: {
								...a.media,
								filename: filename(),
								alt: alt() || null,
							},
						};
					} else {
						return {
							...a,
							spoiler: spoiler(),
							filename: filename(),
							alt: alt() || undefined,
						};
					}
				}
				return a;
			}),
		);

		modalCtl.close();
	};

	return (
		<Modal>
			<form
				style="width:300px"
				onSubmit={(e) => {
					e.preventDefault();
					save();
				}}
			>
				<h2>attachment</h2>
				<div
					style="height:70px;width:100px;background-size:cover;background-position:center;border-radius:4px;margin:8px 0"
					style:background-image={attachment()?.status === "uploaded"
						? `url(${getThumbFromId(attachment()!.media.id, 64)})`
						: "none"}
				>
				</div>
				<label style="display:block;margin:4px 0">
					<h3 class="dim">filename</h3>
					<input
						type="text"
						value={filename()}
						onInput={(e) => setFilename(e.currentTarget.value)}
						style="padding:4px;width:100%;box-sizing:border-box"
					/>
				</label>
				<label style="display:block;margin:4px 0">
					<h3 class="dim">alt text</h3>
					<input
						type="text"
						value={alt()}
						onInput={(e) => setAlt(e.currentTarget.value)}
						placeholder="add a description"
						style="padding:4px;width:100%;box-sizing:border-box"
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
					<Checkbox checked={spoiler()} seed="modal-attachment-spoiler" />
					<label for="opt-spoiler">
						<div>Mark as spoiler</div>
						<div class="dim">
							Hide the attachment behind a clickable overlay
						</div>
					</label>
				</div>
				<Show when={false}>
					<div class="option">
						<input
							id="opt-exif"
							type="checkbox"
							checked={exif()}
							onInput={(e) => setExif(e.currentTarget.checked)}
							style="display:none"
							disabled={false /* TODO: once strip_exif is set to true, it cannot be set to false */}
						/>
						{/* TODO: styles for disabled checkbox */}
						<Checkbox checked={exif()} seed="modal-attachment-exif" />
						<label for="opt-exif">
							<div>Include metadata</div>
							<div class="dim">
								Preserve EXIF data from the original file
							</div>
						</label>
					</div>
				</Show>
				<div class="bottom">
					<button type="button" onClick={() => modalCtl.close()}>cancel</button>
					<button type="submit" class="primary">save</button>
				</div>
			</form>
		</Modal>
	);
};
