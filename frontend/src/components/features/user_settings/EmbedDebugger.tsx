import { createSignal, For, Show } from "solid-js";
import type { Embed, UnfurlerLogEntry } from "ts-sdk";
import { useApi } from "@/api";
import { CodeBlock } from "@/atoms/Markdown";
import { EmbedView } from "@/components/shared/UrlEmbed";

type UnfurlerDebug = {
	embeds: Embed[];
	log: UnfurlerLogEntry[];
};

export const EmbedDebugger = () => {
	const api2 = useApi();
	const [url, setUrl] = createSignal("");
	const [loading, setLoading] = createSignal(false);
	const [data, setData] = createSignal<UnfurlerDebug | null>(null);
	const [error, setError] = createSignal<{ error: string } | null>(null);

	async function generate(e: SubmitEvent) {
		e.preventDefault();
		const u = url();
		if (!u) return;
		try {
			setLoading(true);
			const { data, error } = await api2.client.http.POST(
				"/api/v1/unfurler/debug",
				{
					body: { url: u },
				},
			);
			setData(data ?? null);
			setError(error ?? null);
		} finally {
			setLoading(false);
		}
	}

	return (
		<div>
			<h2>embed debugger</h2>
			<form onSubmit={generate} style="display:flex">
				<label>
					<h3 class="dim">url</h3>
					<input
						type="url"
						onInput={(e) => setUrl(e.currentTarget.value)}
						placeholder="https://example.com"
						ref={(el) => queueMicrotask(() => el.focus())}
					/>
				</label>
				<button
					type="submit"
					class="button primary"
					disabled={loading()}
					style="align-self:end;margin-left:4px"
				>
					{loading() ? "unfurling..." : "unfurl url"}
				</button>
			</form>
			<Show when={error()}>
				<div style="border: solid red 1px;padding: 4px;background: #ff000044;">
					<b>Error:</b> {error()?.error}
				</div>
			</Show>
			<Show when={data()}>
				{(d) => (
					<div>
						<h3 class="dim" style="margin-top:8px">
							log
						</h3>
						<ul style="list-style: disc inside">
							<For each={d().log}>
								{(entry) => (
									<li>
										<code>{JSON.stringify(entry)}</code>
									</li>
								)}
							</For>
						</ul>
						<h3 class="dim" style="margin-top:8px">
							rendered
						</h3>
						<For each={d().embeds}>
							{(embed) => <EmbedView embed={embed} />}
						</For>
						<h3 class="dim" style="margin-top:8px">
							data
						</h3>
						<CodeBlock text={JSON.stringify(d(), null, 4)} lang="json" />
					</div>
				)}
			</Show>
		</div>
	);
};
