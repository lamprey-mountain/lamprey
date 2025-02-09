import { VoidProps } from "solid-js";
import { User } from "sdk";
import { useCtx } from "../context.ts";

export function Info(props: VoidProps<{ user: User }>) {
	const ctx = useCtx();

	const setName = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
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
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
					body: { description },
				});
			},
		});
	};

	return (
		<>
			<h2>info</h2>
			<div>name: {props.user.name}</div>
			<div>description: {props.user.description}</div>
			<div>
				id: <code class="select-all">{props.user.id}</code>
			</div>
			<button onClick={setName}>set name</button>
			<br />
			<button onClick={setDescription}>set description</button>
			<br />
		</>
	);
}
