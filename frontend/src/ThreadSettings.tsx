import { For, Show, VoidProps } from "solid-js";
import { useCtx } from "./context.ts";
import { ThreadT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";

const tabs = [
	{ name: "info", path: "", component: Info },
	// TODO: { name: "invites", path: "invites", component: Invites },
	// TODO: { name: "roles", path: "roles", component: Roles },
	// TODO: { name: "members", path: "members", component: Members },
];

export const ThreadSettings = (props: { thread: ThreadT; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				thread settings: {currentTab()?.name}
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/thread/${props.thread.id}/settings/${tab.path}`}>
									{tab.name}
								</A>
							</li>
						)}
					</For>
				</ul>
			</nav>
			<main>
				<Show when={currentTab()} fallback="unknown page">
					<Dynamic
						component={currentTab()?.component}
						thread={props.thread}
					/>
				</Show>
			</main>
		</div>
	);
};

function Info(props: VoidProps<{ thread: ThreadT }>) {
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
