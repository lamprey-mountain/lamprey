import type { Thread } from "sdk";
import { createSignal, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";

export function Info(props: VoidProps<{ thread: Thread }>) {
	const ctx = useCtx();
	const [editingNsfw, setEditingNsfw] = createSignal(props.thread.nsfw);
	const [editingName, setEditingName] = createSignal(props.thread.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.thread.description,
	);

	const save = () => {
		ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
			params: { path: { thread_id: props.thread.id } },
			body: {
				name: editingName(),
				description: editingDescription(),
				nsfw: editingNsfw(),
			},
		});
	};

	const toggleArchived = () => {
		if (props.thread.archived_at) {
			ctx.client.http.DELETE("/api/v1/thread/{thread_id}/archive", {
				params: { path: { thread_id: props.thread.id } },
			});
		} else {
			ctx.client.http.PUT("/api/v1/thread/{thread_id}/archive", {
				params: { path: { thread_id: props.thread.id } },
			});
		}
	};

	const toggleLocked = () => {
		if (props.thread.locked) {
			ctx.client.http.DELETE("/api/v1/thread/{thread_id}/lock", {
				params: { path: { thread_id: props.thread.id } },
			});
		} else {
			ctx.client.http.PUT("/api/v1/thread/{thread_id}/lock", {
				params: { path: { thread_id: props.thread.id } },
			});
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
				thread id: <code class="select-all">{props.thread.id}</code>
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
					<div>mark this thread as not safe for work</div>
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
						{props.thread.archived_at ? "unarchive" : "archive"}
					</button>
					<span style="margin-left:8px">
						{props.thread.archived_at
							? "shows this thread in the nav bar"
							: "hides this thread in the nav bar"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={toggleLocked}>
						{props.thread.locked ? "unlock" : "lock"}
					</button>
					<span style="margin-left:8px">
						{props.thread.locked
							? "anyone will be able to chat in this thread"
							: "only moderators can chat in this thread"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>remove</button>
					<span style="margin-left:8px">
						archives and locks this thread and hides it from all listings
						(direct links still work)
					</span>
				</label>
				<br />
			</div>
		</>
	);
}
