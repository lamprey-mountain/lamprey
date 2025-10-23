import type { Channel } from "sdk";
import { createSignal, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";

export function Info(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
	const [editingNsfw, setEditingNsfw] = createSignal(props.channel.nsfw);
	const [editingName, setEditingName] = createSignal(props.channel.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.channel.description,
	);

	const save = () => {
		ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: {
				name: editingName(),
				description: editingDescription(),
				nsfw: editingNsfw(),
			},
		});
	};

	const toggleArchived = () => {
		if (props.channel.archived_at) {
			api.channels.unarchive(props.channel.id);
		} else {
			api.channels.archive(props.channel.id);
		}
	};

	const toggleLocked = () => {
		if (props.channel.locked) {
			api.channels.unlock(props.channel.id);
		} else {
			api.channels.lock(props.channel.id);
		}
	};

	return (
		<>
			<h2>info</h2>
			<button onClick={save}>save changes</button>
			<br />
			name
			<br />
			<input
				value={editingName()}
				type="text"
				onInput={(e) => setEditingName(e.target.value)}
			/>
			<br />
			<br />
			description
			<br />
			<textarea onInput={(e) => setEditingDescription(e.target.value)}>
				{editingDescription()}
			</textarea>
			<br />
			<br />
			<div>
				channel id: <code class="select-all">{props.channel.id}</code>
			</div>
			<div>
				<label>
					<div>
						<input
							type="checkbox"
							checked={editingNsfw()}
							onInput={(e) => setEditingNsfw(e.currentTarget.checked)}
						/>
						<b>nsfw</b>
					</div>
					<div>mark this channel as not safe for work</div>
				</label>
			</div>
			<div>(todo) tags</div>
			<div>(todo) visibility</div>
			<br />
			{/* TODO: add padding to all settings */}
			<div class="danger" style="margin:0 2px">
				<h3>danger zone</h3>
				<label>
					{/* should this really be in the "danger zone"? archiving doesnt do much */}
					<button onClick={toggleArchived}>
						{props.channel.archived_at ? "unarchive" : "archive"}
					</button>
					<span style="margin-left:8px">
						{props.channel.archived_at
							? "shows this channel in the nav bar"
							: "hides this channel in the nav bar"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={toggleLocked}>
						{props.channel.locked ? "unlock" : "lock"}
					</button>
					<span style="margin-left:8px">
						{props.channel.locked
							? "anyone will be able to chat in this channel"
							: "only moderators can chat in this channel"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>remove</button>
					<span style="margin-left:8px">
						archives and locks this channel and hides it from all listings
						(direct links still work)
					</span>
				</label>
				<br />
			</div>
		</>
	);
}
