import { createUpload } from "sdk";
import { For, type VoidProps } from "solid-js";
import { useApi2, useEmoji2 } from "@/api";
import { useConfig } from "../../../config.tsx";
import { useCtx } from "../../../context.ts";
import { useModals } from "../../../contexts/modal";
import type { RoomT } from "../../../types.ts";

export function Emoji(props: VoidProps<{ room: RoomT }>) {
	const config = useConfig();
	const api2 = useApi2();
	const emoji2 = useEmoji2();
	const [, modalCtl] = useModals();
	const emoji = emoji2.useRoomList(() => props.room.id);

	function create() {}

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

	return (
		<>
			<h2>custom emoji</h2>
			<form
				style="padding: 8px 0;border: solid #555 1px"
				onSubmit={(e) => {
					e.preventDefault();
					const c = (e.target as HTMLFormElement).querySelectorAll("input");
					const name = c[0].value;
					const file = (c[1] as HTMLInputElement).files?.[0];
					if (!file) return;
					createUpload({
						client: api2.client,
						file,
						onComplete: (media) => {
							api2.client.http.POST("/api/v1/room/{room_id}/emoji", {
								params: {
									path: {
										room_id: props.room.id,
									},
								},
								body: { animated: false, media_id: media.id, name },
							});
						},
						onFail: () => {},
						onPause: () => {},
						onProgress: () => {},
						onResume: () => {},
					});
				}}
			>
				<label>
					name
					<input name="name" type="text" />
				</label>
				<br />
				<label>
					image
					<input name="file" type="file" />
				</label>
				<br />
				<input value="create" type="submit" />
			</form>
			<ul>
				<For each={emoji()?.state.ids ?? []}>
					{(id) => {
						const i = emoji2.cache.get(id);
						if (!i) return null;
						return (
							<li>
								<img
									src={`${config.cdn_url}/emoji/${i.id}`}
									style="height:1em;width:1em"
								/>
								{i.name} <button onClick={() => remove(i.id)}>remove</button>
							</li>
						);
					}}
				</For>
			</ul>
		</>
	);
}
