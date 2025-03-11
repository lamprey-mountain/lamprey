import type { Thread } from "sdk";
import type { VoidProps } from "solid-js";
import { useCtx } from "../context.ts";

export function Info(props: VoidProps<{ thread: Thread }>) {
	const ctx = useCtx();

	const setName = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
					params: { path: { thread_id: props.thread.id } },
					body: { name },
				});
			},
		});
	};

	const setDescription = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "description?",
			cont(description) {
				if (typeof description !== "string") return;
				ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
					params: { path: { thread_id: props.thread.id } },
					body: { description },
				});
			},
		});
	};
	return (
		<>
			<h2>info</h2>
			<div>thread name: {props.thread.name}</div>
			<div>thread description: {props.thread.description}</div>
			<div>
				thread id: <code class="select-all">{props.thread.id}</code>
			</div>
			<button onClick={setName}>set name</button>
			<br />
			<button onClick={setDescription}>set description</button>
			<br />
		</>
	);
}
