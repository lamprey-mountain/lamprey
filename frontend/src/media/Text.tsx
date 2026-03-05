import {
	createEffect,
	createResource,
	createSignal,
	For,
	on,
	Show,
} from "solid-js";
import { formatBytes, getUrl, type MediaProps } from "./util.tsx";
import { useCtx } from "../context.ts";
import { debounce } from "@solid-primitives/scheduled";
import { useModals } from "../contexts/modal";
import { flags } from "../flags";

// 16KiB
const MAX_PREVIEW_SIZE = 16384;

export const TextView = (props: MediaProps) => {
	const ctx = useCtx();

	const ty = () => props.media.content_type.split(";")[0];
	const isHtml = () =>
		props.media.filename.endsWith(".html") ||
		props.media.filename.endsWith(".htm") ||
		props.media.filename.endsWith(".svg") ||
		props.media.content_type.includes("text/html") ||
		props.media.content_type.includes("image/svg+xml");

	const [collapsed, setCollapsed] = createSignal(true);
	const [copied, setCopied] = createSignal(false);
	const [preview, setPreview] = createSignal(false);

	const [text] = createResource(() => props.media, async (media) => {
		const req = await fetch(getUrl(media), {
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
			const [, modalCtl] = useModals();
			modalCtl.alert("file not loaded yet");
		}
	};

	let highlightEl!: HTMLPreElement;

	function highlight() {
		text();
		if (preview()) return;
		if (!highlightEl) return;
		import("highlight.js").then(({ default: hljs }) => {
			// HACK: determine file type via extension
			// HACK: retain line numbers
			for (const el of [...highlightEl.children]) {
				const cell = el as HTMLElement;
				delete cell.dataset.highlighted;
				cell.classList.add(
					"language-" +
						props.media.filename.match(/\.([a-z0-9]+)$/)?.[1],
				);
				hljs.highlightElement(cell);
			}
		});
	}

	createEffect(highlight);

	return (
		<div class="media-text code-block-container">
			<div class="code-block-header">
				<div class="file-info">
					<a class="filename" download={props.media.filename} href={getUrl(props.media)}>
						{props.media.filename}
					</a>
					<span class="dim">{formatBytes(props.media.size)}</span>
				</div>
				<div class="actions">
					<Show when={isHtml() && flags.has("markdown_html_preview")}>
						<button onClick={() => setPreview(!preview())}>
							{preview() ? "code" : "preview"}
						</button>
					</Show>
					<button class="copy" onClick={copy}>
						{copied() ? "copied!" : "copy"}
					</button>
				</div>
			</div>
			<div class="wrap" classList={{ collapsed: collapsed() }}>
				<Show
					when={preview()}
					fallback={
						<pre class="numbered" ref={highlightEl}>
							<For each={text()?.split("\n")}>
								{(l, i) => <code data-line-number={i() + 1}>{l + "\n"}</code>}
							</For>
						</pre>
					}
				>
					<div class="html-preview">
						<iframe srcdoc={text()} sandbox="allow-scripts" />
					</div>
				</Show>
				<Show when={!preview()}>
					<button class="expand-btn" onClick={() => setCollapsed((c) => !c)}>
						{collapsed() ? "expand" : "collapse"}
					</button>
				</Show>
				<Show when={props.media.size > MAX_PREVIEW_SIZE}>
					<div class="warn-truncated">
						<span class="warn">warning:</span> file preview truncated (too long!)
					</div>
				</Show>
			</div>
		</div>
	);
};
