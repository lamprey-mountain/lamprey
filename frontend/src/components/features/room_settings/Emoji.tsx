import { createUpload, type EmojiCustom, type Media } from "sdk";
import {
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
	type VoidProps,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useApi, useEmoji } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Search } from "@/atoms/Search";
import { useModals } from "@/contexts/modal";
import { emojiNameFromFilename, getEmojiUrl } from "@/media/util";
import type { RoomT } from "@/types";
import { icDelete } from "@/utils/icons";

// TODO: edit emoji button -> modal?
// TODO: only animate emoji on hover or when reduce motion not enabled
// TODO: better upload error handling
// TODO: upload progress indicator

export function Emoji(props: VoidProps<{ room: RoomT }>) {
	const api2 = useApi();
	const emoji2 = useEmoji();
	const [, modalCtl] = useModals();
	const [renaming, setRenaming] = createSignal<string | null>(null);
	const [dragging, setDragging] = createSignal(false);
	const emoji = emoji2.useRoomList(() => props.room.id);

	function remove(emoji_id: string) {
		modalCtl.confirm("really remove?", (confirmed) => {
			if (!confirmed) return;
			api2.client.http.DELETE("/api/v1/room/{room_id}/emoji/{emoji_id}", {
				params: {
					path: {
						room_id: props.room.id,
						emoji_id,
					},
				},
			});
		});
	}

	function rename(emoji: EmojiCustom, name: string) {
		if (!name) return;
		if (emoji.name === name) return;

		api2.client.http.PATCH("/api/v1/room/{room_id}/emoji/{emoji_id}", {
			params: {
				path: {
					room_id: props.room.id,
					emoji_id: emoji.id,
				},
			},
			body: { name },
		});
	}

	let uploadRef!: HTMLInputElement;
	let skipModal = false;

	const create = (e: MouseEvent) => {
		skipModal = e.shiftKey;
		uploadRef.click();
	};

	const createEmoji = (media: Media) => {
		api2.client.http
			.POST("/api/v1/room/{room_id}/emoji", {
				params: { path: { room_id: props.room.id } },
				body: {
					animated: false,
					media_id: media.id,
					name: emojiNameFromFilename(media.filename),
				},
			})
			.catch(console.error);
	};

	const uploadFile = (file: File, skip = false) => {
		createUpload({
			client: api2.client,
			file,
			onComplete: (media) => {
				if (skip) createEmoji(media);
				else
					modalCtl.open({
						type: "emoji_upload",
						room_id: props.room.id,
						media,
					});
			},
			onFail: () => {},
			onPause: () => {},
			onProgress: () => {},
			onResume: () => {},
		});
	};

	const upload = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
		for (const file of Array.from(e.currentTarget.files ?? [])) {
			uploadFile(file, skipModal);
		}
		skipModal = false;
		e.currentTarget.value = "";
	};

	const hasFiles = (e: DragEvent) =>
		Array.from(e.dataTransfer?.types ?? []).includes("Files");

	// listen on window so dropping anywhere in the open settings modal works
	onMount(() => {
		let dragCounter = 0;

		const onDragEnter = (e: DragEvent) => {
			if (!hasFiles(e)) return;
			dragCounter++;
			setDragging(true);
		};

		const onDragOver = (e: DragEvent) => {
			if (!hasFiles(e)) return;
			e.preventDefault();
			if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
			setDragging(true);
		};

		const onDragLeave = () => {
			dragCounter--;
			if (dragCounter <= 0) {
				dragCounter = 0;
				setDragging(false);
			}
		};

		const onDrop = (e: DragEvent) => {
			if (!hasFiles(e)) return;
			e.preventDefault();
			dragCounter = 0;
			setDragging(false);
			for (const file of Array.from(e.dataTransfer?.files ?? [])) {
				uploadFile(file, e.shiftKey);
			}
		};

		window.addEventListener("dragenter", onDragEnter);
		window.addEventListener("dragover", onDragOver);
		window.addEventListener("dragleave", onDragLeave);
		window.addEventListener("drop", onDrop);

		onCleanup(() => {
			window.removeEventListener("dragenter", onDragEnter);
			window.removeEventListener("dragover", onDragOver);
			window.removeEventListener("dragleave", onDragLeave);
			window.removeEventListener("drop", onDrop);
		});
	});

	return (
		<div class="room-settings-emoji">
			<h2>custom emoji</h2>
			<div class="header">
				<Search placeholder="Search emoji..." />
				<button class="button primary" onClick={create}>
					upload
				</button>
				<input
					name="file"
					type="file"
					multiple
					style="display:none"
					ref={uploadRef}
					onInput={upload}
				/>
			</div>
			<ul class="emojis">
				<For each={emoji()?.state.ids ?? []}>
					{(id) => {
						const i = emoji2.cache.get(id);
						if (!i) return null;
						return (
							<li class="item">
								<img class="emoji" src={getEmojiUrl(i.id)} />
								<Show
									when={renaming() === i.id}
									fallback={
										<div class="name" onClick={() => setRenaming(i.id)}>
											{i.name}
										</div>
									}
								>
									<input
										value={i.name}
										type="text"
										class="name-input"
										onBlur={(e) => {
											rename(i, e.currentTarget.value);
											setRenaming(null);
										}}
										onKeyDown={(e) => {
											if (e.key === "Enter") {
												rename(i, e.currentTarget.value);
												setRenaming(null);
											}
										}}
										ref={(el) =>
											queueMicrotask(() => {
												el.focus();
												el.select();
											})
										}
									/>
								</Show>
								<div style="flex:1"></div>
								<menu class="menu">
									<button
										type="button"
										class="button icon-button danger"
										onClick={() => remove(i.id)}
									>
										<Icon src={icDelete} />
									</button>
								</menu>
							</li>
						);
					}}
				</For>
			</ul>
			<Portal mount={document.getElementById("overlay")!}>
				<Show when={dragging()}>
					<div class="dnd-upload-message">
						<div class="inner">
							<div>drop to upload emoji</div>
							<div class="dim">(hold shift to upload directly)</div>
						</div>
					</div>
				</Show>
			</Portal>
		</div>
	);
}
