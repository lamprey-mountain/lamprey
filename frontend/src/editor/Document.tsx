import type { Channel } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	on,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { createEditor } from "./DocumentEditor.tsx";
import type { DiffMark } from "./diff-plugin.ts";
import icBranchDefault from "../assets/edit.png";
import icBranchPrivate from "../assets/edit.png";
import icBranchNew from "../assets/edit.png";
import icBranchFork from "../assets/edit.png";
import icBranch from "../assets/edit.png";
import icMergeFull from "../assets/edit.png";
import icMergeCherrypick from "../assets/edit.png";
import icFormatBold from "../assets/format-bold.png";
import icFormatItalic from "../assets/format-italic.png";
import icFormatCode from "../assets/format-code.png";
import icFormatStrikethrough from "../assets/format-strikethrough.png";
import icFormatUrl from "../assets/format-url.png";
import { useDocument } from "../contexts/document.tsx";
import { useModals } from "../contexts/modal.tsx";
import { TextSelection } from "prosemirror-state";
import { useChannel } from "../contexts/channel.tsx";
import { useApi } from "../api.tsx";
import type { HistoryPagination } from "sdk";
import * as Y from "yjs";
import { base64UrlDecode } from "./editor-utils.ts";
import { Time } from "../Time.tsx";
import { schema } from "./schema.ts";
import { md } from "../markdown_utils.tsx";
import { DOMParser, type Node as PMNode } from "prosemirror-model";
import { diffWords } from "diff";
import type { Tokens } from "marked";

type ChangesetSelection = {
	start_seq: number;
	end_seq: number;
};

type DocumentProps = {
	channel: Channel;
};

export const Document = (
	props: DocumentProps & {
		selectedSeq: ChangesetSelection | null;
		onSelectChangeset: (changeset: ChangesetSelection | null) => void;
		hoverSeq: ChangesetSelection | null;
		onHoverChangeset: (changeset: ChangesetSelection | null) => void;
	},
) => {
	const [branchId, setBranchId] = createSignal(props.channel.id);
	const [editor, setEditor] = createSignal<any>(null);

	return (
		<div class="document">
			<DocumentHeader
				channel={props.channel}
				editor={editor}
			/>
			<DocumentMain
				channel={props.channel}
				setEditor={setEditor}
				editor={() => editor()}
				selectedSeq={props.selectedSeq}
				onSelectChangeset={props.onSelectChangeset}
				hoverSeq={props.hoverSeq}
				onHoverChangeset={props.onHoverChangeset}
			/>
		</div>
	);
};

const DocumentHeader = (
	props: DocumentProps & {
		editor: any;
	},
) => {
	const [doc, update] = useDocument();
	const [, modalCtl] = useModals();
	const [ch, setCh] = useChannel()!;
	const [active, setActive] = createSignal<
		"branches" | "merge" | "export" | "insert" | null
	>(null);

	const toggleHistory = () => {
		setCh("history_view", !ch.history_view);
	};

	const applyFormat = (wrap: string) => {
		const view = props.editor?.view;
		if (!view) return;

		const { from, to } = view.state.selection;
		if (from === to) return;

		const len = wrap.length;
		const tr = view.state.tr;

		const textBefore = tr.doc.textBetween(from - len, from);
		const textAfter = tr.doc.textBetween(to, to + len);
		const isWrapped = textBefore === wrap && textAfter === wrap;

		if (isWrapped) {
			tr.delete(to, to + len);
			tr.delete(from - len, from);
		} else {
			tr.insertText(wrap, to);
			tr.insertText(wrap, from);
			tr.setSelection(TextSelection.create(tr.doc, from + len, to + len));
		}

		view.dispatch(tr);
		view.focus();
	};

	const openLinkModal = () => {
		if (props.editor) {
			modalCtl.open({ type: "link", editor: props.editor });
		}
	};

	const [branchBtn, setBranchBtn] = createSignal<HTMLElement>();
	const [branchMenu, setBranchMenu] = createSignal<HTMLElement>();
	const branchPos = useFloating(branchBtn, branchMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-start",
		middleware: [offset(4), flip(), shift()],
	});

	const [mergeBtn, setMergeBtn] = createSignal<HTMLElement>();
	const [mergeMenu, setMergeMenu] = createSignal<HTMLElement>();
	const mergePos = useFloating(mergeBtn, mergeMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-start",
		middleware: [offset(4), flip(), shift()],
	});

	const [exportBtn, setExportBtn] = createSignal<HTMLElement>();
	const [exportMenu, setExportMenu] = createSignal<HTMLElement>();
	const exportPos = useFloating(exportBtn, exportMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-start",
		middleware: [offset(4), flip(), shift()],
	});

	const [insertBtn, setInsertBtn] = createSignal<HTMLElement>();
	const [insertMenu, setInsertMenu] = createSignal<HTMLElement>();
	const insertPos = useFloating(insertBtn, insertMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-start",
		middleware: [offset(4), flip(), shift()],
	});

	onMount(() => {
		const close = () => setActive(null);
		window.addEventListener("click", close);
		onCleanup(() => window.removeEventListener("click", close));
	});

	return (
		<header>
			<div class="menu-group">
				<button
					ref={setBranchBtn}
					onClick={(e) => {
						e.stopPropagation();
						setActive(active() === "branches" ? null : "branches");
					}}
					classList={{ active: active() === "branches" }}
				>
					branches
				</button>
				<Show when={true}>
					<button
						ref={setMergeBtn}
						onClick={(e) => {
							e.stopPropagation();
							setActive(active() === "merge" ? null : "merge");
						}}
						classList={{ active: active() === "merge" }}
					>
						merge
					</button>
				</Show>
				<button
					onClick={(e) => {
						e.stopPropagation();
						toggleHistory();
					}}
				>
					history
				</button>
			</div>
			<div class="menu-group">
				<button
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("**");
					}}
				>
					<img class="icon" src={icFormatBold} />
				</button>
				<button
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("*");
					}}
				>
					<img class="icon" src={icFormatItalic} />
				</button>
				<button
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("~~");
					}}
				>
					<img class="icon" src={icFormatStrikethrough} />
				</button>
				<button
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("`");
					}}
				>
					<img class="icon" src={icFormatCode} />
				</button>
				<button
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						openLinkModal();
					}}
				>
					<img class="icon" src={icFormatUrl} />
				</button>
				<button
					ref={setInsertBtn}
					onClick={(e) => {
						e.stopPropagation();
						setActive(active() === "insert" ? null : "insert");
					}}
					classList={{ active: active() === "insert" }}
				>
					insert
				</button>
			</div>
			<div class="menu-group">
				<button
					ref={setExportBtn}
					onClick={(e) => {
						e.stopPropagation();
						setActive(active() === "export" ? null : "export");
					}}
					classList={{ active: active() === "export" }}
				>
					export
				</button>
			</div>

			<Show when={active() === "branches"}>
				<Portal>
					<menu
						class="branch-menu document-menu"
						ref={setBranchMenu}
						style={{
							position: branchPos.strategy,
							top: `${branchPos.y ?? 0}px`,
							left: `${branchPos.x ?? 0}px`,
							"z-index": 100,
						}}
						onClick={(e) => e.stopPropagation()}
					>
						<input
							type="text"
							placeholder="filter branches..."
							style="margin:4px 8px;padding:2px 4px;border-radius:2px"
							ref={(el) => queueMicrotask(() => el.focus())}
						/>
						<ul>
							<li
								class="default"
								classList={{ selected: doc.branchId === props.channel.id }}
								onClick={() => {
									update("branchId", props.channel.id);
									setActive(null);
								}}
							>
								<button>
									<img class="icon" src={icBranchDefault} />
									<div class="info">
										<div>default</div>
										<div class="dim">the main/master/default branch</div>
									</div>
								</button>
							</li>
							<li>
								<button>
									<img class="icon" src={icBranch} />
									<div class="info">
										<div>branch name here</div>
										<div class="dim">
											created by <b>@user</b> n minutes ago
										</div>
									</div>
								</button>
							</li>
							<li class="private">
								<button>
									<img class="icon" src={icBranchPrivate} />
									<div class="info">
										<div>branch name here</div>
										<div class="dim">
											private branch; created n minutes ago
										</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li class="new">
								<button>
									<img class="icon" src={icBranchNew} />
									<div class="info">
										<div>new</div>
										<div class="dim">create a new branch</div>
									</div>
								</button>
							</li>
							<li class="new">
								<button>
									<img class="icon" src={icBranchFork} />
									<div class="info">
										<div>new from changes</div>
										<div class="dim">
											create a new branch from existing changes
										</div>
									</div>
								</button>
							</li>
							<li class="new">
								<button>
									<img class="icon" src={icBranchFork} />
									<div class="info">
										<div>new private</div>
										<div class="dim">
											create a new private branch only visible to you
										</div>
									</div>
								</button>
							</li>
						</ul>
					</menu>
				</Portal>
			</Show>
			<Show when={active() === "merge"}>
				<Portal>
					<menu
						class="merge-menu document-menu"
						ref={setMergeMenu}
						style={{
							position: mergePos.strategy,
							top: `${mergePos.y ?? 0}px`,
							left: `${mergePos.x ?? 0}px`,
							"z-index": 100,
						}}
						onClick={(e) => e.stopPropagation()}
					>
						<ul>
							<li>
								<button>
									<img class="icon" src={icMergeFull} />
									<div class="info">
										<div>full</div>
										<div class="dim">
											fully merge all changes in this branch
										</div>
									</div>
								</button>
							</li>
							<li>
								<button>
									<img class="icon" src={icMergeCherrypick} />
									<div class="info">
										<div>cherry pick</div>
										<div class="dim">view diff; merge specific changes</div>
									</div>
								</button>
							</li>
						</ul>
					</menu>
				</Portal>
			</Show>
			<Show when={active() === "export"}>
				<Portal>
					<menu
						class="export-menu document-menu"
						ref={setExportMenu}
						style={{
							position: exportPos.strategy,
							top: `${exportPos.y ?? 0}px`,
							left: `${exportPos.x ?? 0}px`,
							"z-index": 100,
						}}
						onClick={(e) => e.stopPropagation()}
					>
						<ul>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>{false ? "open in new tab" : "publish document"}</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>download as html</div>
										<div class="dim">single file .mhtml file</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>download as markdown</div>
									</div>
								</button>
							</li>
						</ul>
					</menu>
				</Portal>
			</Show>
			<Show when={active() === "insert"}>
				<Portal>
					<menu
						class="insert-menu document-menu"
						ref={setInsertMenu}
						style={{
							position: insertPos.strategy,
							top: `${insertPos.y ?? 0}px`,
							left: `${insertPos.x ?? 0}px`,
							"z-index": 100,
						}}
						onClick={(e) => e.stopPropagation()}
					>
						<ul>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>media</div>
										<div class="dim">insert images, videos, and audio</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>table</div>
										<div class="dim">insert tables with rows and columns</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>code</div>
										<div class="dim">
											insert code blocks with syntax highlighting
										</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>symbols</div>
										<div class="dim">insert special characters and symbols</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => setActive(null)}>
									<div class="info">
										<div>time</div>
										<div class="dim">insert current date and time</div>
									</div>
								</button>
							</li>
						</ul>
					</menu>
				</Portal>
			</Show>
		</header>
	);
};

type Editor = ReturnType<typeof createEditor>;

const DocumentMain = (
	props: DocumentProps & {
		setEditor: (editor: Editor | null) => void;
		selectedSeq: ChangesetSelection | null;
		onSelectChangeset: (changeset: ChangesetSelection | null) => void;
		hoverSeq: ChangesetSelection | null;
		onHoverChangeset: (changeset: ChangesetSelection | null) => void;
		editor: () => Editor | null;
	},
) => {
	const api = useApi();
	const [diffLoading, setDiffLoading] = createSignal(false);
	const [history, setHistory] = createSignal<HistoryPagination | null>(null);
	const [currentRevision, setCurrentRevision] = createSignal<number | null>(
		null,
	);
	const [previewRevision, setPreviewRevision] = createSignal<number | null>(
		null,
	);
	const editor = createMemo(() => props.editor());
	const [viewReady, setViewReady] = createSignal(false);

	// Determine mode: 'edit' | 'diff_preview' | 'diff_readonly'
	// Hover preview takes priority when actively hovering; otherwise show selected
	const mode = () => {
		if (props.hoverSeq !== null) return "diff_preview";
		if (props.selectedSeq !== null) return "diff_readonly";
		return "edit";
	};

	// Get the changeset info for the current revision
	const currentChangeset = () => {
		// Hover takes priority over selected when hovering
		const selection = props.hoverSeq ?? props.selectedSeq;
		if (selection === null) return null;
		const hist = history();
		if (!hist) return null;
		return hist.changesets.find(
			(cs) =>
				cs.start_seq === selection.start_seq &&
				cs.end_seq === selection.end_seq,
		) ?? null;
	};

	// Load history when channel changes
	createEffect(
		on(() => props.channel.id, async (channelId) => {
			setEditState(null);
			api.documents.clearChannelCache(channelId);

			try {
				const data = await api.documents.history(channelId, channelId, {
					limit: 50,
					by_author: false,
					by_changes: 100,
					by_tag: true,
					by_time: 60 * 5,
				});
				setHistory(data);
			} catch (e) {
				console.error("Failed to load history:", e);
			}
		}),
	);

	const ed = createEditor({
		diffMode: () => mode() !== "edit",
	});

	onMount(() => {
		props.setEditor(ed);
	});

	const [editState, setEditState] = createSignal<any>(null);
	const [lastSubscribedChannel, setLastSubscribedChannel] = createSignal<
		string | null
	>(null);

	// Subscribe to channel only when channel actually changes
	createEffect(() => {
		const chId = props.channel.id;
		const ed = editor();
		const m = mode();

		if (!ed || m !== "edit") return;
		if (lastSubscribedChannel() === chId) return;

		ed.subscribe(chId, chId);
		setLastSubscribedChannel(chId);
	});

	// Handle readonly/preview mode: load historical revision
	createEffect(() => {
		const ed = editor();
		const m = mode();
		if (!ed || m === "edit") return;

		const selection = m === "diff_readonly"
			? props.selectedSeq
			: props.hoverSeq;
		if (selection === null) return;

		// Save edit state before switching to readonly (only if not already saved)
		if (editState() === null) {
			setEditState(ed.view.state);
		}

		loadReadonlyRevision(ed, selection, m === "diff_preview");
	});

	// Clear preview when returning to edit mode
	createEffect(() => {
		const ed = editor();
		const m = mode();
		if (!ed || m !== "edit") return;

		ed.setDiffMarks([]);
		setCurrentRevision(null);
		setPreviewRevision(null);

		// Restore edit state if we have one saved
		const savedState = editState();
		if (savedState) {
			ed.setState(savedState);
			setEditState(null);
		}
	});

	// Helper to load a readonly historical revision
	const loadReadonlyRevision = async (
		ed: Editor,
		selection: ChangesetSelection,
		isPreview: boolean = false,
	) => {
		// Use end_seq for the "after" state (the revision showing the changeset's result)
		const afterSeq = selection.end_seq;
		// Use start_seq - 1 for the "before" state (document before this changeset)
		const beforeSeq = Math.max(0, selection.start_seq - 1);

		const targetRevision = isPreview ? previewRevision() : currentRevision();
		const revisionId = `${props.channel.id}@${afterSeq}`;

		const cachedSerdoc = api.documents.revisionCache.get(revisionId);
		const hasCache = cachedSerdoc !== undefined;

		if (targetRevision === afterSeq && hasCache) return;

		if (!hasCache) setDiffLoading(true);

		try {
			let newSerdoc: any = cachedSerdoc;
			if (!newSerdoc) {
				newSerdoc = await api.documents.getRevisionContent(
					props.channel.id,
					revisionId,
				);
				if (!newSerdoc) return;
			}

			// Abort Guard: Check if user moved away while fetching
			const activeSelection = isPreview ? props.hoverSeq : props.selectedSeq;
			if (
				activeSelection?.start_seq !== selection.start_seq ||
				activeSelection?.end_seq !== selection.end_seq
			) return;

			// Fetch previous revision for diff
			let oldSerdoc: any = null;
			if (beforeSeq > 0) {
				const prevRevisionId = `${props.channel.id}@${beforeSeq}`;
				oldSerdoc = api.documents.revisionCache.get(prevRevisionId) ?? null;
				if (!oldSerdoc) {
					oldSerdoc = await api.documents.getRevisionContent(
						props.channel.id,
						prevRevisionId,
					);
					if (!oldSerdoc) return;
				}
			}

			// Abort Guard 2
			const activeSelectionPostFetch = isPreview
				? props.hoverSeq
				: props.selectedSeq;
			if (
				activeSelectionPostFetch?.start_seq !== selection.start_seq ||
				activeSelectionPostFetch?.end_seq !== selection.end_seq
			) return;

			// Compute diff marks BEFORE setting state
			const marks = computeDiffMarks(oldSerdoc ?? {}, newSerdoc);

			// Safely call createReadonlyState depending on what is exported by Editor
			const newHtml = serdocToHtml(newSerdoc);
			const readonlyState =
				typeof (ed as any).createReadonlyStateFromHtml === "function"
					? (ed as any).createReadonlyStateFromHtml(newHtml)
					: ed.createReadonlyState(newHtml);

			ed.setState(readonlyState);
			ed.setDiffMarks(marks);

			if (isPreview) {
				setPreviewRevision(afterSeq);
			} else {
				setCurrentRevision(afterSeq);
			}
		} catch (e) {
			console.error("Failed to load revision:", e);
		} finally {
			const activeSelection = isPreview ? props.hoverSeq : props.selectedSeq;
			if (
				activeSelection?.start_seq === selection.start_seq &&
				activeSelection?.end_seq === selection.end_seq && !hasCache
			) {
				setDiffLoading(false);
			}
		}
	};

	onCleanup(() => {
		props.setEditor(null);
	});

	const [restoreMenuOpen, setRestoreMenuOpen] = createSignal(false);
	const [restoreBtn, setRestoreBtn] = createSignal<HTMLElement>();
	const [restoreMenu, setRestoreMenu] = createSignal<HTMLElement>();
	const restorePos = useFloating(restoreBtn, restoreMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-end",
		middleware: [offset(4), flip(), shift()],
	});

	onMount(() => {
		const close = () => setRestoreMenuOpen(false);
		window.addEventListener("click", close);
		onCleanup(() => window.removeEventListener("click", close));
	});

	const handleRestoreVersion = async (mode: "current" | "new") => {
		const selection = props.selectedSeq;
		if (selection === null) return;
		setRestoreMenuOpen(false);

		// Use end_seq as the revision to restore
		const seq = selection.end_seq;

		try {
			if (mode === "new") {
				const newBranchName = `restored-${
					new Date().toISOString().slice(0, 10)
				}`;
				await api.client.http.POST(
					"/api/v1/document/{channel_id}/branch/{parent_id}/fork",
					{
						params: {
							path: {
								channel_id: props.channel.id,
								parent_id: props.channel.id,
							},
						},
						body: {
							name: newBranchName,
							description: `Restored from revision @${seq}`,
							private: false,
						},
					},
				);
				console.log("Created restored branch:", newBranchName);
			} else {
				console.log("Restore to current branch @seq:", seq);
			}
		} catch (e) {
			console.error("Failed to restore version:", e);
		}
	};

	// Extract headings from serdoc for table of contents
	const [headings, setHeadings] = createSignal<
		{ level: number; text: string }[]
	>([]);

	// Extract headings from markdown content
	const extractHeadingsFromMarkdown = (markdown: string) => {
		console.log(
			"[TOC] extractHeadingsFromMarkdown called, markdown length:",
			markdown.length,
		);
		const result: { level: number; text: string }[] = [];
		const tokens = md.lexer(markdown);
		console.log("[TOC] tokens:", tokens.length);

		for (const token of tokens) {
			console.log("[TOC] token type:", token.type);
			if (token.type === "heading") {
				const heading = token as Tokens.Heading;
				const text = heading.tokens
					?.map((t) => (t.type === "text" ? t.text : ""))
					.join("")
					.trim() ?? heading.text.trim();
				console.log("[TOC] found heading:", heading.depth, text);
				if (text) {
					result.push({ level: heading.depth, text });
				}
			}
		}
		console.log("[TOC] result:", result);
		return result;
	};

	// Extract headings from ProseMirror document by scanning text content
	const extractHeadingsFromDoc = () => {
		const ed = editor();
		if (!ed || !ed.view) return [];

		const state = ed.view.state;
		const doc = state.doc;
		const result: { level: number; text: string }[] = [];

		doc.descendants((node) => {
			if (node.isBlock) {
				const textContent = node.textContent;
				if (textContent) {
					// Split by newline because blocks can have multiple lines
					const lines = textContent.split("\n");
					for (const line of lines) {
						const text = line.trim();
						// Check for ATX-style headings: # Heading, ## Subheading, etc.
						const match = text.match(/^(#{1,6})\s+(.+)$/);
						if (match) {
							const level = match[1].length;
							const headingText = match[2].trim();
							if (headingText) {
								result.push({ level, text: headingText });
							}
						}
					}
				}
				return false; // Skip traversing into inline children, we only need block textContent
			}
			return true;
		});

		console.log("[TOC] extractHeadingsFromDoc result:", result);
		return result;
	};

	// Update headings when selection changes (historical revision)
	createEffect(() => {
		console.log("[TOC] historical effect running");
		const selection = props.selectedSeq ?? props.hoverSeq;
		const seq = selection?.end_seq ?? null;
		console.log("[TOC] selection:", selection, "seq:", seq);

		if (!seq) {
			console.log("[TOC] no seq, returning");
			return;
		}

		const serdoc = api.documents.revisionCache.get(
			`${props.channel.id}@${seq}`,
		);
		console.log("[TOC] serdoc from cache:", serdoc);
		if (!serdoc) {
			console.log("[TOC] no serdoc, returning");
			return;
		}

		const doc = serdoc?.data ?? serdoc;
		console.log("[TOC] doc:", doc);
		if (!doc?.root?.blocks) {
			console.log("[TOC] no root.blocks, returning");
			return;
		}

		for (const block of doc.root.blocks) {
			console.log("[TOC] block:", block);
			if (block.Markdown?.content) {
				console.log("[TOC] found markdown content");
				setHeadings(extractHeadingsFromMarkdown(block.Markdown.content));
				return;
			}
		}
		console.log("[TOC] no markdown content found");
		setHeadings([]);
	});

	// Update headings from live editor content when not viewing history
	createEffect(() => {
		console.log("[TOC] live effect running");
		const selection = props.selectedSeq ?? props.hoverSeq;
		console.log("[TOC] selection:", selection);
		if (selection) {
			console.log("[TOC] has selection, skipping live update");
			return; // Don't update from live content when viewing history
		}

		// Wait for view to be ready
		const ready = viewReady();
		console.log("[TOC] viewReady:", ready);
		if (!ready) {
			console.log("[TOC] view not ready, returning");
			return;
		}

		const ed = editor();
		console.log("[TOC] editor:", ed);
		if (!ed) {
			console.log("[TOC] no editor, returning");
			return;
		}

		const view = ed.view;
		console.log("[TOC] view:", view);
		if (!view) {
			console.log("[TOC] no view, returning");
			return;
		}

		console.log("[TOC] view.dom:", view.dom);

		// Track editor state changes using dispatchTransaction
		const updateHeadings = () => {
			console.log("[TOC] updateHeadings called from dispatch");
			setHeadings(extractHeadingsFromDoc());
		};

		const originalDispatch = view.dispatch.bind(view);
		view.dispatch = function (tr) {
			originalDispatch(tr);
			if (tr.docChanged) {
				console.log("[TOC] doc changed, updating headings");
				updateHeadings();
			}
		};
		console.log("[TOC] hooked into dispatch");

		// Initial update
		updateHeadings();

		return () => {
			console.log("[TOC] cleanup live effect");
			// Restore original dispatch
			view.dispatch = originalDispatch;
		};
	});

	const scrollToHeading = (targetText: string) => {
		const ed = editor();
		if (!ed) return;
		const view = ed.view;
		if (!view) return;

		const state = view.state;
		const doc = state.doc;
		let targetPos = -1;

		doc.descendants((node, pos) => {
			if (targetPos !== -1) return false;

			if (node.isBlock) {
				const text = node.textContent;
				if (!text) return false;

				const lines = text.split("\n");
				let currentOffset = 0;

				for (const line of lines) {
					const match = line.trim().match(/^(#{1,6})\s+(.+)$/);
					if (match && match[2].trim() === targetText) {
						// Found the matching line! Map its string index to a ProseMirror position.
						const stringIndex = text.indexOf(line, currentOffset);

						let pmOffset = 0;
						let strOffset = 0;

						// Iterate over the block's inline children to map string offset -> PM offset properly
						// (Required because custom Atoms like @mentions take 1 position but have N characters of text)
						node.forEach((child) => {
							if (strOffset >= stringIndex) return;

							const childTextLen = child.textContent.length;
							if (strOffset + childTextLen >= stringIndex) {
								// The target text falls inside this inline child
								pmOffset += stringIndex - strOffset;
								strOffset = stringIndex;
							} else {
								pmOffset += child.nodeSize;
								strOffset += childTextLen;
							}
						});

						targetPos = pos + 1 + pmOffset;
						return false; // Stop iterating descendants completely
					}
					currentOffset += line.length + 1; // +1 to account for the split newline char
				}
				return false; // Skip inline children
			}
			return true;
		});

		if (targetPos !== -1) {
			try {
				// Get viewport coordinates of the exact text position
				const coords = view.coordsAtPos(targetPos);
				window.scrollTo({
					top: window.scrollY + coords.top - 80, // -80px offset so it clears floating headers
					behavior: "smooth",
				});
			} catch (e) {
				console.error("Failed to scroll to heading position:", e);
			}
		}
	};

	return (
		<div>
			<div class="document-left-rail">
				<Show when={mode() === "diff_readonly" && currentChangeset()}>
					{(changeset) => (
						<div class="diff-view-message">
							<div class="diff-view-info">
								Viewing revision from{" "}
								<Time date={new Date(changeset().start_time)} />
							</div>
							<div class="diff-view-actions">
								<button
									class="secondary linkstyled"
									onClick={() => {
										props.onSelectChangeset(null);
										props.onHoverChangeset(null);
									}}
								>
									Cancel
								</button>
								<button
									ref={setRestoreBtn}
									class="secondary"
									onClick={(e) => {
										e.stopPropagation();
										setRestoreMenuOpen(!restoreMenuOpen());
									}}
								>
									Restore ▼
								</button>
							</div>
						</div>
					)}
				</Show>
				{(() => {
					const h = headings();
					console.log("[TOC] render, headings:", h);
					return (
						<Show when={h.length > 0}>
							<div class="document-toc">
								<h4>Table of Contents</h4>
								<ul>
									<For each={h}>
										{(heading) => (
											<li
												style={{
													"margin-left": `${(heading.level - 1) * 12}px`,
												}}
												onClick={() => scrollToHeading(heading.text)}
											>
												{heading.text}
											</li>
										)}
									</For>
								</ul>
							</div>
						</Show>
					);
				})()}
				<Show when={restoreMenuOpen()}>
					<Portal>
						<menu
							class="restore-menu document-menu"
							ref={setRestoreMenu}
							style={{
								position: restorePos.strategy,
								top: `${restorePos.y ?? 0}px`,
								left: `${restorePos.x ?? 0}px`,
								"z-index": 100,
							}}
							onClick={(e) => e.stopPropagation()}
						>
							<ul>
								<li>
									<button onClick={() => handleRestoreVersion("current")}>
										<div class="info">
											<div>Restore to current branch</div>
											<div class="dim">overwrite current content</div>
										</div>
									</button>
								</li>
								<li>
									<button onClick={() => handleRestoreVersion("new")}>
										<div class="info">
											<div>Create new branch</div>
											<div class="dim">fork from this revision</div>
										</div>
									</button>
								</li>
							</ul>
						</menu>
					</Portal>
				</Show>
			</div>
			<main>
				{(() => {
					const ed = editor();
					if (!ed) return null;
					onMount(() => {
						console.log("[TOC] View mounted, setting viewReady");
						setViewReady(true);
						return () => {
							console.log("[TOC] View unmounting, clearing viewReady");
							setViewReady(false);
						};
					});
					return (
						<ed.View
							onSubmit={() => false}
							channelId={props.channel.id}
							submitOnEnter={false}
							placeholder={mode() === "edit"
								? "write something cool..."
								: mode() === "diff_readonly"
								? "viewing historical revision (readonly)"
								: ""}
							disabled={mode() !== "edit" || diffLoading()}
						/>
					);
				})()}
			</main>
		</div>
	);
};

// Extracted utility to securely map pure text to absolute PM structure positions
function getDocTextAndMap(doc: PMNode): { text: string; posMap: number[] } {
	let text = "";
	const posMap: number[] = [];

	doc.descendants((node, pos) => {
		if (node.isText) {
			const str = node.text!;
			for (let i = 0; i < str.length; i++) {
				posMap.push(pos + i);
			}
			text += str;
		} else if (node.isBlock) {
			// Represent block boundaries as newlines to give diffWords paragraph context
			if (text.length > 0) {
				posMap.push(pos);
				text += "\n";
			}
		}
		return true; // continue traversing
	});

	return { text, posMap };
}

function mapTextPosToPMPos(
	posMap: number[],
	textPos: number,
	doc: PMNode,
): number {
	if (textPos < 0) return 1;
	// Max valid position is the end of the root node content
	const maxPos = Math.max(1, doc.content.size - 1);
	if (textPos >= posMap.length) return maxPos;
	return posMap[textPos] ?? maxPos;
}

function computeDiffMarks(oldSerdoc: any, newSerdoc: any): DiffMark[] {
	const oldDoc = serdocToProseMirrorDoc(oldSerdoc);
	const newDoc = serdocToProseMirrorDoc(newSerdoc);

	if (!oldDoc || !newDoc) return [];

	const oldData = getDocTextAndMap(oldDoc);
	const newData = getDocTextAndMap(newDoc);

	const changes = diffWords(oldData.text, newData.text);

	const marks: DiffMark[] = [];
	let oldTextPos = 0;
	let newTextPos = 0;

	for (const change of changes) {
		const len = change.value.length;

		if (change.added) {
			const from = mapTextPosToPMPos(newData.posMap, newTextPos, newDoc);
			const to = mapTextPosToPMPos(newData.posMap, newTextPos + len, newDoc);
			if (from < to) {
				marks.push({ type: "insertion", from, to });
			}
			newTextPos += len;
		} else if (change.removed) {
			const pos = mapTextPosToPMPos(newData.posMap, newTextPos, newDoc);
			// Replace actual newlines so deletion widgets don't aggressively line-break visually
			const cleanText = change.value.replace(/\n/g, " ↵ ");
			marks.push({ type: "deletion", pos, text: cleanText });
			oldTextPos += len;
		} else {
			oldTextPos += len;
			newTextPos += len;
		}
	}

	return marks;
}

function serdocToProseMirrorDoc(serdoc: any) {
	try {
		const doc = serdoc?.data ?? serdoc;
		if (!doc) return null;

		if (doc?.root?.blocks) {
			const htmlParts: string[] = [];
			for (const block of doc.root.blocks) {
				if (block.Markdown?.content) {
					htmlParts.push(block.Markdown.content);
				}
			}
			if (htmlParts.length === 0) {
				htmlParts.push("<p></p>");
			}
			const div = document.createElement("div");
			div.innerHTML = htmlParts.join("");
			return DOMParser.fromSchema(schema).parse(div);
		}

		if (typeof doc === "string") {
			const div = document.createElement("div");
			div.innerHTML = doc;
			return DOMParser.fromSchema(schema).parse(div);
		}

		return null;
	} catch (e) {
		console.error("Failed to parse serdoc:", e, serdoc);
		return null;
	}
}

function serdocToHtml(serdoc: any): string {
	try {
		const doc = serdoc?.data ?? serdoc;
		if (!doc) return "<p></p>";

		if (doc?.root?.blocks) {
			const htmlParts: string[] = [];
			for (const block of doc.root.blocks) {
				if (block.Markdown?.content) {
					htmlParts.push(block.Markdown.content);
				}
			}
			if (htmlParts.length === 0) return "<p></p>";
			return htmlParts.join("\n\n");
		}

		if (typeof doc === "string") return doc;

		return "<p></p>";
	} catch (e) {
		console.error("Failed to convert serdoc to HTML:", e, serdoc);
		return "<p></p>";
	}
}
