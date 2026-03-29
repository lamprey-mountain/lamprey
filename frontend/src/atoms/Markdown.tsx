import { useNavigate } from "@solidjs/router";
import type { Token, Tokens } from "marked";
import type { Channel } from "sdk";
import {
	createContext,
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	onMount,
	type ParentProps,
	Show,
	Switch,
	useContext,
} from "solid-js";
import {
	useApi2,
	useChannels2,
	useRoles2,
	useRoomMembers2,
	useUsers2,
} from "@/api";
import { useUserPopout } from "../contexts/mod";
import { getTwemoji } from "../emoji";
import { flags } from "../flags";
import { md } from "../markdown_utils";
import { getEmojiUrl } from "../media/util";

// --- Context ---

const MarkdownContext = createContext<{ channel?: Channel }>();

// --- Components ---

function UserMention(props: { id: string }) {
	const ctx = useContext(MarkdownContext);
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();
	const { userView, setUserView } = useUserPopout();
	const user = users2.use(() => props.id);
	const room_member = createMemo(() => {
		if (!ctx?.channel?.room_id) return null;
		return roomMembers2.cache.get(`${ctx.channel!.room_id!}:${props.id}`);
	});

	return (
		<span
			class="mention mention-user"
			onClick={(e) => {
				e.stopPropagation();
				const currentTarget = e.currentTarget as HTMLElement;
				if (userView()?.ref === currentTarget) {
					setUserView(null);
				} else {
					setUserView({
						user_id: props.id as any,
						room_id: ctx?.channel?.room_id as any,
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
	const roles2 = useRoles2();
	const role = createMemo(() => {
		if (!ctx?.channel?.room_id) return null;
		return roles2.cache.get(props.id);
	});

	return <span class="mention mention-role">@{role()?.name ?? "..."}</span>;
}

function ChannelMention(props: { id: string }) {
	const channels2 = useChannels2();
	const navigate = useNavigate();
	const channel = channels2.use(() => props.id);

	return (
		<span
			class="mention mention-channel"
			onClick={(e) => {
				e.stopPropagation();
				navigate(`/channel/${props.id}`);
			}}
		>
			#{channel()?.name ?? "unknown channel"}
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

function Spoiler(props: { tokens: Token[] }) {
	const [shown, setShown] = createSignal(false);
	return (
		<span
			class="spoiler"
			classList={{ shown: shown() }}
			onClick={(e) => {
				e.stopPropagation();
				setShown(!shown());
			}}
		>
			<RenderTokens tokens={props.tokens} />
		</span>
	);
}

function CodeBlock(props: { text: string; lang?: string }) {
	let ref!: HTMLElement;

	const [copied, setCopied] = createSignal(false);
	const [preview, setPreview] = createSignal(false);

	createEffect(() => {
		if (!preview() && ref) {
			import("highlight.js").then(({ default: hljs }) => {
				if (ref) {
					delete (ref as any).dataset.highlighted;
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
							<button onClick={() => setPreview(!preview())}>
								{preview() ? "code" : "preview"}
							</button>
						</Show>
						<Show when={isRust() && flags.has("markdown_rust_playground")}>
							<button onClick={openPlayground}>play</button>
						</Show>
						<button onClick={copy}>{copied() ? "copied" : "copy"}</button>
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
	const escape = (html: string) => {
		return html
			.replace(/&/g, "&amp;")
			.replace(/</g, "&lt;")
			.replace(/>/g, "&gt;")
			.replace(/"/g, "&quot;")
			.replace(/'/g, "&#39;");
	};

	const html = createMemo(() => getTwemoji(escape(props.text)));

	return <span innerHTML={html()} />;
}

// --- Renderer ---

function RenderTokens(props: { tokens?: Token[] }) {
	return (
		<For each={props.tokens}>{(token) => <TokenView token={token} />}</For>
	);
}

function TokenView(props: { token: Token }) {
	return (
		<Switch>
			<Match when={props.token.type === "paragraph"}>
				<p>
					<RenderTokens tokens={(props.token as Tokens.Paragraph).tokens} />
				</p>
			</Match>
			<Match when={props.token.type === "text"}>
				<Show
					when={(props.token as Tokens.Text).tokens}
					fallback={<TwemojiText text={(props.token as Tokens.Text).text} />}
				>
					<RenderTokens tokens={(props.token as Tokens.Text).tokens} />
				</Show>
			</Match>
			<Match when={props.token.type === "blockquote"}>
				<blockquote>
					<RenderTokens tokens={(props.token as Tokens.Blockquote).tokens} />
				</blockquote>
			</Match>
			<Match when={props.token.type === "code"}>
				<CodeBlock
					text={(props.token as Tokens.Code).text}
					lang={(props.token as Tokens.Code).lang}
				/>
			</Match>
			<Match when={props.token.type === "list"}>
				<Show
					when={(props.token as Tokens.List).ordered}
					fallback={
						<ul>
							<For each={(props.token as Tokens.List).items}>
								{(item) => (
									<li>
										<RenderTokens tokens={item.tokens} />
									</li>
								)}
							</For>
						</ul>
					}
				>
					<ol start={(props.token as Tokens.List).start || 1}>
						<For each={(props.token as Tokens.List).items}>
							{(item) => (
								<li>
									<RenderTokens tokens={item.tokens} />
								</li>
							)}
						</For>
					</ol>
				</Show>
			</Match>
			<Match when={props.token.type === "heading"}>
				<DynamicHeading
					depth={(props.token as Tokens.Heading).depth}
					tokens={(props.token as Tokens.Heading).tokens}
				/>
			</Match>
			<Match when={props.token.type === "strong"}>
				<strong>
					<RenderTokens tokens={(props.token as Tokens.Strong).tokens} />
				</strong>
			</Match>
			<Match when={props.token.type === "em"}>
				<em>
					<RenderTokens tokens={(props.token as Tokens.Em).tokens} />
				</em>
			</Match>
			<Match when={props.token.type === "del"}>
				<del>
					<RenderTokens tokens={(props.token as Tokens.Del).tokens} />
				</del>
			</Match>
			<Match when={props.token.type === "codespan"}>
				<code>{(props.token as Tokens.Codespan).text}</code>
			</Match>
			<Match when={props.token.type === "link"}>
				<a
					href={(props.token as Tokens.Link).href}
					title={(props.token as Tokens.Link).title ?? undefined}
					target="_blank"
					rel="noopener noreferrer"
				>
					<RenderTokens tokens={(props.token as Tokens.Link).tokens} />
				</a>
			</Match>
			<Match when={props.token.type === "image"}>
				<img
					src={(props.token as Tokens.Image).href}
					alt={(props.token as Tokens.Image).text}
					title={(props.token as Tokens.Image).title ?? undefined}
				/>
			</Match>
			<Match when={props.token.type === "br"}>
				<br />
			</Match>
			<Match when={props.token.type === "hr"}>
				<hr />
			</Match>
			<Match when={props.token.type === "html"}>
				<TwemojiText text={(props.token as Tokens.HTML).text} />
			</Match>

			{/* Custom Extensions */}
			<Match when={props.token.type === "spoiler"}>
				<Spoiler tokens={(props.token as any).tokens} />
			</Match>
			<Match when={props.token.type === "mention"}>
				<MentionToken token={props.token as any} />
			</Match>
		</Switch>
	);
}

function DynamicHeading(props: { depth: number; tokens: Token[] }) {
	return (
		<Switch>
			<Match when={props.depth === 1}>
				<h1>
					<RenderTokens tokens={props.tokens} />
				</h1>
			</Match>
			<Match when={props.depth === 2}>
				<h2>
					<RenderTokens tokens={props.tokens} />
				</h2>
			</Match>
			<Match when={props.depth === 3}>
				<h3>
					<RenderTokens tokens={props.tokens} />
				</h3>
			</Match>
			<Match when={props.depth === 4}>
				<h4>
					<RenderTokens tokens={props.tokens} />
				</h4>
			</Match>
			<Match when={props.depth === 5}>
				<h5>
					<RenderTokens tokens={props.tokens} />
				</h5>
			</Match>
			<Match when={props.depth === 6}>
				<h6>
					<RenderTokens tokens={props.tokens} />
				</h6>
			</Match>
		</Switch>
	);
}

function MentionToken(props: { token: any }) {
	return (
		<Switch>
			<Match when={props.token.mention_type === "user"}>
				<UserMention id={props.token.id} />
			</Match>
			<Match when={props.token.mention_type === "role"}>
				<RoleMention id={props.token.id} />
			</Match>
			<Match when={props.token.mention_type === "channel"}>
				<ChannelMention id={props.token.id} />
			</Match>
			<Match when={props.token.mention_type === "emoji"}>
				<CustomEmoji
					id={props.token.id}
					name={props.token.name}
					animated={props.token.animated}
				/>
			</Match>
		</Switch>
	);
}

// --- Exported Component ---

export const Markdown = (
	props: ParentProps<{
		content: string;
		channel_id?: string;
		inline?: boolean;
		class?: string;
		classList?: { [k: string]: boolean | undefined };
		ref?: HTMLElement | ((el: HTMLElement) => void);
	}>,
) => {
	const channels2 = useChannels2();
	const channel = channels2.use(() => props.channel_id);

	const tokens = createMemo(() => {
		const t = md.lexer(props.content);
		if (props.inline) {
			if (t.length === 1 && t[0].type === "paragraph") {
				return (t[0] as Tokens.Paragraph).tokens;
			}
		}
		return t;
	});

	return (
		<MarkdownContext.Provider value={{ channel: channel() }}>
			<div
				class={`markdown ${props.class ?? ""}`}
				classList={props.classList}
				ref={props.ref as any}
			>
				<RenderTokens tokens={tokens()} />
				{props.children}
			</div>
		</MarkdownContext.Provider>
	);
};
