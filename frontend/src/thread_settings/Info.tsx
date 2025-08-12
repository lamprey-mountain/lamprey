import type { Thread } from "sdk";
import { createSignal, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";

export function Info(props: VoidProps<{ thread: Thread }>) {
	const ctx = useCtx();
	const [editingName, setEditingName] = createSignal(props.thread.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.thread.description,
	);

	const save = () => {
		ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
			params: { path: { thread_id: props.thread.id } },
			body: { name: editingName(), description: editingDescription() },
		});
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
			<br />
			<div>(todo) tags</div>
			<div>(todo) locked</div>
			<div>(todo) archived</div>
			<div>(todo) visibility</div>
			<br />
		</>
	);
}
