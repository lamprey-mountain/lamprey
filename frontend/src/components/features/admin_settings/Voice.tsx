import { createResource, For, Suspense } from "solid-js";
import { useApi } from "@/api";
import { CodeBlock } from "@/atoms/Markdown";

export const Voice = () => {
	const api = useApi();

	const [status] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/server/@self/voice");
		return data;
	});

	// TODO: better rendering/styling

	return (
		<>
			<h2>Voice</h2>
			<section class="section admin-settings-voice">
				<h3>Selective forwarding units (sfus)</h3>
				<Suspense fallback="loading">
					<div>
						<For each={status()?.sfus ?? []} fallback="no sfus">
							{(sfu) => (
								<CodeBlock text={JSON.stringify(sfu, null, 4)} lang="json" />
							)}
						</For>
					</div>
				</Suspense>
			</section>
		</>
	);
};
