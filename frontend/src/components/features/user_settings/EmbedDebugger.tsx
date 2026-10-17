import { createSignal, For, Show } from "solid-js";
import type { Embed } from "ts-sdk";
import { useApi } from "@/api";
import { CodeBlock } from "@/atoms/Markdown";
import { EmbedView } from "@/components/shared/UrlEmbed";

type LogLevel = "Trace" | "Debug" | "Info" | "Warning" | "Error";

// TEMP: sdk types are stale, manually port crate-common/src/v1/types/unfurl.rs for now
type LogEntry = {
	id: number;
	started: number;
	ended: number;
	level: LogLevel;
	children?: LogEntry[];
} & (
	| {
			type: "Http";
			url: string;
			status: number;
			headers: [string, string][];
			body_preview?: string | null;
	  }
	| {
			type: "Other";
			message: string;
			attributes?: Record<string, string>;
	  }
);

type UnfurlerDebug = {
	embeds: Embed[];
	log: LogEntry[];
};

const formatAttributes = (attributes?: Record<string, string>) =>
	attributes
		? Object.entries(attributes)
				.map(([key, value]) => `${key}=${value}`)
				.join(" ")
		: "";

const LogEntryView = (props: { entry: LogEntry }) => {
	const detail = () =>
		props.entry.type === "Http"
			? props.entry.body_preview
			: formatAttributes(props.entry.attributes);

	return (
		<Show when={props.entry.level !== "Trace"}>
			<li>
				<div>
					{/* TODO: color log level, probably copy redex logs */}
					<span style="display:inline-block;width:64px;text-align:right;color:oklch(var(--color-fg5))">
						{props.entry.level}
					</span>{" "}
					<span>
						{props.entry.type === "Http"
							? `HTTP ${props.entry.status} ${props.entry.url}`
							: props.entry.message}
					</span>{" "}
					{/* <span class="dim">{props.entry.ended - props.entry.started}ms</span> */}
					<Show when={detail()}>{(d) => <span class="dim"> {d()}</span>}</Show>
				</div>
				<Show when={props.entry.children?.length}>
					<ul style="padding-left:16px">
						<For each={props.entry.children}>
							{(child) => <LogEntryView entry={child} />}
						</For>
					</ul>
				</Show>
			</li>
		</Show>
	);
};

export const EmbedDebugger = () => {
	const api = useApi();
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
			const { data, error } = await api.client.http.POST(
				"/api/v1/unfurler/debug",
				{
					body: { url: u },
				},
			);
			// TODO: remove cast once the openapi schema is regenerated
			setData((data as UnfurlerDebug | undefined) ?? null);
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
							rendered
						</h3>
						<For each={d().embeds}>
							{(embed) => <EmbedView embed={embed} />}
						</For>
						<h3 class="dim" style="margin-top:8px">
							log
						</h3>
						<ul>
							<For each={d().log}>
								{(entry) => <LogEntryView entry={entry} />}
							</For>
						</ul>
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
