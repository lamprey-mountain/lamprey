import { useNavigate } from "@solidjs/router";
import type { Channel } from "sdk";
import {
	createContext,
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	type ParentProps,
	Show,
	Switch,
	useContext,
} from "solid-js";
import { useChannels, useRoles, useRoomMembers, useUsers } from "@/api";
import { useUserPopout } from "@/contexts/mod";
import { getEmojiHex } from "@/lib/emoji";
import { flags } from "@/lib/flags";
import { Parser, loaded } from "@/lib/markdown";
import { getEmojiUrl } from "@/media/util";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { Dynamic } from "solid-js/web";
import {
	MentionData,
	SerializedBlock,
	SerializedDocument,
	SerializedInline,
} from "@/lib/markdown/ast";

// --- Context ---

const MarkdownContext = createContext<{
	channel?: Channel;
	allowDiffTags?: boolean;
}>();

// --- Components ---

function UserMention(props: { id: string }) {
	const ctx = useContext(MarkdownContext);
	const users2 = useUsers();
	const roomMembers2 = useRoomMembers();
	const { userView, setUserView } = useUserPopout();
	const user = users2.use(() => props.id);
	const room_member = createMemo(() => {
		if (!ctx?.channel?.room_id) return null;
		return roomMembers2.cache.get(`${ctx.channel?.room_id!}:${props.id}`);
	});

	return (
		<span
			class="mention mention-user"
			onClick={(e) => {
				e.stopPropagation();
				e.preventDefault();
				const currentTarget = e.currentTarget as HTMLElement;
				if (userView()?.ref === currentTarget) {
					setUserView(null);
				} else {
					setUserView({
						user_id: props.id,
						room_id: ctx?.channel?.room_id ?? undefined,
						channel_id: ctx?.channel?.id,
						ref: currentTarget,
						source: "message",
					});
				}
			}}
		>
			@{room_member()?.override_name ?? user()?.name ?? "unknown user"}
		</span>
	);
}

function RoleMention(props: { id: string }) {
	const ctx = useContext(MarkdownContext);
	const roles2 = useRoles();
	const role = createMemo(() => {
		if (!ctx?.channel?.room_id) return null;
		return roles2.cache.get(props.id);
	});

	return <span class="mention mention-role">@{role()?.name ?? "..."}</span>;
}

function ChannelMention(props: { id: string }) {
	const channels2 = useChannels();
	const navigate = useNavigate();
	const channel = channels2.use(() => props.id);

	return (
		<span
			class="mention mention-channel"
			onClick={(e) => {
				e.stopPropagation();
				e.preventDefault();
				navigate(`/channel/${props.id}`);
			}}
		>
			#{channel()?.name ?? "unknown channel"}
		</span>
	);
}

function EveryoneMention() {
	return (
		<span
			class="mention mention-everyone"
			onClick={(e) => {
				e.stopPropagation();
				e.preventDefault();
				// TODO: do something on click?
			}}
		>
			@everyone
		</span>
	);
}

function CustomEmoji(props: { id: string; name: string; animated?: boolean }) {
	return (
		<img
			class="emoji"
			src={getEmojiUrl(props.id)}
			alt={`:${props.name}:`}
			title={`:${props.name}:`}
		/>
	);
}

function Spoiler(props: { children: SerializedInline[] }) {
	const [shown, setShown] = createSignal(false);
	return (
		<span
			class="spoiler"
			classList={{ shown: shown() }}
			onClick={(e) => {
				e.stopPropagation();
				e.preventDefault();
				setShown(!shown());
			}}
		>
			<For each={props.children}>
				{(child) => <RenderInline inline={child} />}
			</For>
		</span>
	);
}

function CodeBlock(props: { text: string; lang?: string | null }) {
	let ref!: HTMLElement;

	const [copied, setCopied] = createSignal(false);
	const [preview, setPreview] = createSignal(false);

	createEffect(() => {
		if (!preview() && ref) {
			import("highlight.js").then(({ default: hljs }) => {
				if (ref) {
					delete ref.dataset.highlighted;
					hljs.highlightElement(ref);
				}
			});
		}
	});

	const copy = () => {
		navigator.clipboard.writeText(props.text);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	const isHtml = () =>
		props.lang === "html" ||
		props.lang === "htm" ||
		props.lang === "xml" ||
		props.lang === "svg";

	const isRust = () => props.lang === "rust" || props.lang === "rs";

	const openPlayground = () => {
		const url = `https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&code=${encodeURIComponent(
			props.text,
		)}`;
		window.open(url, "_blank");
	};

	return (
		<Show
			when={flags.has("markdown_code_components")}
			fallback={
				<pre>
					<code ref={ref} class={props.lang ? `language-${props.lang}` : ""}>
						{props.text}
					</code>
				</pre>
			}
		>
			<div class="code-block-container">
				<div class="code-block-header">
					<span class="lang">{props.lang ?? "text"}</span>
					<div class="actions">
						<Show when={isHtml() && flags.has("markdown_html_preview")}>
							<button
								type="button"
								class="button"
								onClick={() => setPreview(!preview())}
							>
								{preview() ? "code" : "preview"}
							</button>
						</Show>
						<Show when={isRust() && flags.has("markdown_rust_playground")}>
							<button type="button" class="button" onClick={openPlayground}>
								play
							</button>
						</Show>
						<button type="button" class="button" onClick={copy}>
							{copied() ? "copied" : "copy"}
						</button>
					</div>
				</div>
				<Show
					when={preview()}
					fallback={
						<pre>
							<code
								ref={ref}
								class={props.lang ? `language-${props.lang}` : ""}
							>
								{props.text}
							</code>
						</pre>
					}
				>
					<div class="html-preview">
						<iframe srcdoc={props.text} sandbox="allow-scripts" />
					</div>
				</Show>
			</div>
		</Show>
	);
}

function TwemojiText(props: { text: string }) {
	const ctx = useContext(MarkdownContext);

	const escape = (html: string) => {
		let escaped = html
			.replace(/&/g, "&amp;")
			.replace(/</g, "&lt;")
			.replace(/>/g, "&gt;")
			.replace(/"/g, "&quot;")
			.replace(/'/g, "&#39;");

		if (ctx?.allowDiffTags) {
			escaped = escaped
				.replace(/\uE000/g, "<ins>")
				.replace(/\uE001/g, "</ins>")
				.replace(/\uE002/g, "<del>")
				.replace(/\uE003/g, "</del>");
		}

		return escaped;
	};

	const html = createMemo(() => getTwemoji(escape(props.text)));

	return <span innerHTML={html()} />;
}

// --- Renderers ---

function RenderBlock(props: { block: SerializedBlock }) {
	return (
		<Switch>
			<Match when={props.block.type === "Header" && props.block}>
				{(b) => (
					<Dynamic component={`h${b().level}`}>
						<For each={b().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</Dynamic>
				)}
			</Match>
			<Match when={props.block.type === "Paragraph" && props.block}>
				{(b) => (
					<p>
						<For each={b().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</p>
				)}
			</Match>
			<Match when={props.block.type === "Blockquote" && props.block}>
				{(b) => (
					<blockquote>
						<For each={b().children}>
							{(child) => <RenderBlock block={child} />}
						</For>
					</blockquote>
				)}
			</Match>
			<Match when={props.block.type === "Codeblock" && props.block}>
				{(b) => <CodeBlock text={b().content} lang={b().language} />}
			</Match>
			<Match when={props.block.type === "List" && props.block}>
				{(b) => (
					<ul>
						<For each={b().items}>{(item) => <RenderBlock block={item} />}</For>
					</ul>
				)}
			</Match>
			<Match when={props.block.type === "ListItem" && props.block}>
				{(b) => (
					<li>
						<For each={b().content}>
							{(child) => <RenderBlock block={child} />}
						</For>
					</li>
				)}
			</Match>
			<Match when={props.block.type === "Table" && props.block}>
				{(b) => (
					<div style={{ "overflow-x": "auto" }}>
						<table>
							<thead>
								<tr>
									<For each={b().header}>
										{(cell) => (
											<th>
												<For each={cell}>
													{(inline) => <RenderInline inline={inline} />}
												</For>
											</th>
										)}
									</For>
								</tr>
							</thead>
							<tbody>
								<For each={b().rows}>
									{(row) => (
										<tr>
											<For each={row}>
												{(cell) => (
													<td>
														<For each={cell}>
															{(inline) => <RenderInline inline={inline} />}
														</For>
													</td>
												)}
											</For>
										</tr>
									)}
								</For>
							</tbody>
						</table>
					</div>
				)}
			</Match>
		</Switch>
	);
}

const matchMention = <T extends MentionData["type"]>(
	m: MentionData,
	t: T,
): (MentionData & { type: T }) | null => {
	return m.type === t ? (m as MentionData & { type: T }) : null;
};

function RenderInline(props: { inline: SerializedInline }) {
	return (
		<Switch>
			<Match when={props.inline.type === "Strong" && props.inline}>
				{(i) => (
					<strong>
						<For each={i().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</strong>
				)}
			</Match>
			<Match when={props.inline.type === "Emphasis" && props.inline}>
				{(i) => (
					<em>
						<For each={i().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</em>
				)}
			</Match>
			<Match when={props.inline.type === "Strikethrough" && props.inline}>
				{(i) => (
					<del>
						<For each={i().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</del>
				)}
			</Match>
			<Match when={props.inline.type === "Link" && props.inline}>
				{(i) => (
					<a href={i().href} target="_blank" rel="noopener noreferrer">
						<For each={i().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</a>
				)}
			</Match>
			<Match when={props.inline.type === "Spoiler" && props.inline}>
				{(i) => <Spoiler children={i().children} />}
			</Match>
			<Match when={props.inline.type === "Code" && props.inline}>
				{(i) => (
					<code>
						<For each={i().children}>
							{(child) => <RenderInline inline={child} />}
						</For>
					</code>
				)}
			</Match>
			<Match when={props.inline.type === "Text" && props.inline}>
				{(i) => <TwemojiText text={i().content} />}
			</Match>
			<Match when={props.inline.type === "Mention" && props.inline}>
				{(i) => (
					<Switch>
						<Match when={matchMention(i().mention, "User")}>
							{(m) => <UserMention id={m().id} />}
						</Match>
						<Match when={matchMention(i().mention, "Role")}>
							{(m) => <RoleMention id={m().id} />}
						</Match>
						<Match when={matchMention(i().mention, "Channel")}>
							{(m) => <ChannelMention id={m().id} />}
						</Match>
						<Match when={matchMention(i().mention, "Everyone")}>
							{(_) => <EveryoneMention />}
						</Match>
					</Switch>
				)}
			</Match>
			<Match when={props.inline.type === "CustomEmoji" && props.inline}>
				{(i) => (
					<CustomEmoji id={i().id} name={i().name} animated={i().animated} />
				)}
			</Match>
			<Match when={props.inline.type === "UnicodeEmoji" && props.inline}>
				{(i) => <UnicodeEmoji hex={getEmojiHex(i().content)} />}
			</Match>
		</Switch>
	);
}

// --- Exported Component ---

export type MarkdownProps = {
	content: string;
	channel_id?: string;
	inline?: boolean;
	kindaInline?: boolean;
	class?: string;
	classList?: { [k: string]: boolean | undefined };
	allowDiffFormatting?: boolean;
	ref?: HTMLElement | ((el: HTMLElement) => void);
};

export const Markdown = (props: ParentProps<MarkdownProps>) => {
	const channels2 = useChannels();
	const channel = channels2.use(() => props.channel_id);

	const [parser, setParser] = createSignal<Parser>();
	createEffect(() => {
		loaded.then(() => {
			setParser(new Parser());
		});
	});

	const ast = createMemo(() => {
		const p = parser();
		if (!p) return null;
		const parsed = p.parse(props.content);
		return parsed.ast() as SerializedDocument;
	});

	// TODO: use Suspense here?
	return (
		<MarkdownContext.Provider
			value={{ channel: channel(), allowDiffTags: props.allowDiffFormatting }}
		>
			<Show when={ast()}>
				<Dynamic
					component={props.inline ? "span" : "div"}
					class={`markdown ${props.class ?? ""}`}
					classList={props.classList}
					ref={props.ref as any}
				>
					<For each={ast()?.blocks}>
						{(block) => <RenderBlock block={block} />}
					</For>
					{props.children}
				</Dynamic>
			</Show>
		</MarkdownContext.Provider>
	);
};
