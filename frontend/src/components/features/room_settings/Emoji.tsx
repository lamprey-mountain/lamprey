import { createUpload, type EmojiCustom } from "sdk";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi, useEmoji } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Search } from "@/atoms/Search";
import { useModals } from "@/contexts/modal";
import { getEmojiUrl } from "@/media/util";
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

	const create = () => {
		uploadRef.click();
	};

	const upload = (e: InputEvent) => {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		createUpload({
			client: api2.client,
			file,
			onComplete: (media) => {
				modalCtl.open({ type: "emoji_upload", room_id: props.room.id, media });
			},
			onFail: () => {},
			onPause: () => {},
			onProgress: () => {},
			onResume: () => {},
		});
	};

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
		</div>
	);
}
