import { createResource, createSignal, For, Show } from "solid-js";
import { byteFmt, MediaProps } from "./util.ts";
import { useCtx } from "../context.ts";
import { debounce } from "@solid-primitives/scheduled";

// 16KiB
const MAX_PREVIEW_SIZE = 16384;

export const TextView = (props: MediaProps) => {
	const ctx = useCtx();

	const ty = () => props.media.source.mime.split(";")[0];

	const [collapsed, setCollapsed] = createSignal(true);
	const [copied, setCopied] = createSignal(false);

	const [text] = createResource(() => props.media, async (media) => {
		const req = await fetch(media.source.url, {
			headers: {
				"Range": `bytes=0-${MAX_PREVIEW_SIZE}`,
			},
		});
		if (!req.ok) throw req.statusText;
		const text = await req.text();
		return text;
	});

	const unsetCopied = debounce(() => setCopied(false), 1000);
	const copy = () => {
		const t = text();
		if (t) {
			setCopied(true);
			navigator.clipboard.writeText(t);
			unsetCopied();
		} else {
			ctx.dispatch({ do: "modal.alert", text: "file not loaded yet" });
		}
	};

	return (
		<div class="media-text">
			<div class="wrap" classList={{ collapsed: collapsed() }}>
				<button class="copy" onClick={copy}>
					{copied() ? "copied!" : "copy"}
				</button>
				<pre class="numbered">
					<For each={text()?.split("\n")}>{l =>
						<code>{l + "\n"}</code>
					}</For>
				</pre>
				<button onClick={() => setCollapsed((c) => !c)}>
					{collapsed() ? "expand" : "collapse"}
				</button>
				<Show when={props.media.source.size > MAX_PREVIEW_SIZE}>
					<span class="warn">warning:</span> file preview truncated (too long!)
				</Show>
			</div>
			<a download={props.media.filename} href={props.media.source.url}>
				download {props.media.filename}
			</a>
			<div class="dim">{ty()} - {byteFmt.format(props.media.source.size)}</div>
		</div>
	);
};
