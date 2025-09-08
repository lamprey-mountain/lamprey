import { For, type VoidProps } from "solid-js";
import type { RoomT } from "../types.ts";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { createUpload } from "sdk";
import { useConfig } from "../config.tsx";

export function Emoji(props: VoidProps<{ room: RoomT }>) {
	const config = useConfig();
	const api = useApi();
	const ctx = useCtx();
	const emoji = api.emoji.list(() => props.room.id);

	function create() {
	}

	function remove(emoji_id: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really remove?",
			cont(confirmed) {
				if (!confirmed) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/emoji/{emoji_id}", {
					params: {
						path: {
							room_id: props.room.id,
							emoji_id,
						},
					},
				});
			},
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
						client: api.client,
						file,
						onComplete: (media) => {
							api.client.http.POST("/api/v1/room/{room_id}/emoji", {
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
				<For each={emoji()?.items ?? []}>
					{(i) => (
						<li>
							<img
								src={`${config.cdn_url}/emoji/${i.id}`}
								style="height:1em;width:1em"
							/>
							{i.name} <button onClick={() => remove(i.id)}>remove</button>
						</li>
					)}
				</For>
			</ul>
		</>
	);
}
