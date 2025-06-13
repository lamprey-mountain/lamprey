import { For, type VoidProps } from "solid-js";
import { type User } from "sdk";
import { useApi } from "../api.tsx";
import { createResource } from "solid-js";

export function Applications(_props: VoidProps<{ user: User }>) {
	const api = useApi();

	async function create() {
		const name = prompt("name");
		await api.client.http.POST("/api/v1/app", {
			body: {
				name,
				bridge: false,
				public: false,
			},
		});
	}

	const [list] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/app", {
			params: { limit: 100 },
		});
		return data;
	});

	return (
		<>
			<h2>applications</h2>
			<button onClick={create}>create</button>
			<For each={list()?.items ?? []}>
				{(app) => {
					return (
						<div style="border: solid #444 1px">
							<pre>{JSON.stringify(app, null, 2)}</pre>
						</div>
					);
				}}
			</For>
		</>
	);
}
