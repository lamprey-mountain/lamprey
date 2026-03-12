import type { Channel } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
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
import { diffWords } from "diff";
import { schema } from "./schema.ts";
import { md } from "../markdown_utils.tsx";
import { DOMParser } from "prosemirror-model";

type DocumentProps = {
	channel: Channel;
};

export const Document = (
	props: DocumentProps & {
		selectedSeq: number | null;
		onSelectChangeset: (seq: number | null) => void;
		hoverSeq: number | null;
		onHoverChangeset: (seq: number | null) => void;
	},
) => {
	const [branchId, setBranchId] = createSignal(props.channel.id);
	const [editor, setEditor] = createSignal<any>(null);
	// setup ydoc here, pass to DocumentMain?

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

	// top: title, topic(?), notifications, members, search
	// bottom: branches (merge, diff), edit, format, insert, view, tools
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
								<button
									onClick={() => {
										// TODO: publishing documents
										setActive(null);
									}}
								>
									<div class="info">
										<div>{false ? "open in new tab" : "publish document"}</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li>
								<button
									onClick={() => {
										// TODO: download as html
										setActive(null);
									}}
								>
									<div class="info">
										<div>download as html</div>
										<div class="dim">single file .mhtml file</div>
									</div>
								</button>
							</li>
							<li>
								<button
									onClick={() => {
										// TODO: download as markdown (how do i handle media?)
										setActive(null);
									}}
								>
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
		selectedSeq: number | null;
		onSelectChangeset: (seq: number | null) => void;
		hoverSeq: number | null;
		onHoverChangeset: (seq: number | null) => void;
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
	let hoverDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Determine mode: 'edit' | 'diff_preview' | 'diff_readonly'
	const mode = () => {
		if (props.selectedSeq !== null) return "diff_readonly";
		if (props.hoverSeq !== null) return "diff_preview";
		return "edit";
	};

	// Get the changeset info for the current revision
	const currentChangeset = () => {
		const seq = props.selectedSeq ?? props.hoverSeq;
		if (seq === null) return null;
		const hist = history();
		if (!hist) return null;
		return hist.changesets.find((cs) => cs.start_seq === seq) ?? null;
	};

	// Load history when channel changes
	createEffect(
		on(() => props.channel.id, async (channelId) => {
			// Clear edit state
			setEditState(null);

			// Clear the documents revision cache for this channel
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

	// Create editor once on mount
	onMount(() => {
		const ed = createEditor({
			diffMode: () => mode() !== "edit",
		});
		props.setEditor(ed);
	});

	// Store the edit state to restore when exiting diff view
	const [editState, setEditState] = createSignal<any>(null);

	// Track last subscribed channel to avoid duplicate subscriptions
	const [lastSubscribedChannel, setLastSubscribedChannel] = createSignal<
		string | null
	>(null);

	// Subscribe to channel only when channel actually changes
	createEffect(() => {
		const chId = props.channel.id;
		const ed = editor();
		const m = mode();

		// Only subscribe in edit mode
		if (!ed || m !== "edit") {
			return;
		}

		// Skip if already subscribed to this channel
		if (lastSubscribedChannel() === chId) {
			return;
		}

		ed.subscribe(chId, chId);
		setLastSubscribedChannel(chId);
	});

	// Handle readonly/preview mode: load historical revision with debounce for hover
	const [pendingPreviewSeq, setPendingPreviewSeq] = createSignal<number | null>(
		null,
	);

	createEffect(() => {
		const ed = editor();
		const m = mode();
		if (!ed || m === "edit") return;

		const seq = m === "diff_readonly" ? props.selectedSeq : props.hoverSeq;
		if (seq === null) return;

		// Save edit state before switching to readonly (only if not already saved)
		if (editState() === null) {
			setEditState(ed.view.state);
		}

		// Debounce hover previews (150ms) to avoid flickering
		if (m === "diff_preview") {
			// Cancel any pending load
			if (hoverDebounceTimer) {
				clearTimeout(hoverDebounceTimer);
			}

			// Set pending seq for tracking
			setPendingPreviewSeq(seq);

			hoverDebounceTimer = setTimeout(() => {
				// Only load if still hovering at this seq
				if (pendingPreviewSeq() === seq) {
					loadReadonlyRevision(ed, seq, true);
				}
				hoverDebounceTimer = null;
			}, 150);

			return () => {
				if (hoverDebounceTimer) {
					clearTimeout(hoverDebounceTimer);
					hoverDebounceTimer = null;
				}
			};
		}

		// Clear pending preview when switching to readonly
		setPendingPreviewSeq(null);

		// No debounce for readonly (click) mode
		loadReadonlyRevision(ed, seq, false);
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

		// Don't re-subscribe here - the subscribe effect above handles it
	});

	// Helper to load a readonly historical revision (without syncing to server)
	const loadReadonlyRevision = async (
		ed: Editor,
		seq: number,
		isPreview: boolean = false,
	) => {
		const targetRevision = isPreview ? previewRevision() : currentRevision();
		const revisionId = `${props.channel.id}@${seq}`;

		// Check if we have cached content - if so, load immediately without loading state
		const cachedSerdoc = api.documents.revisionCache.get(revisionId);
		const hasCache = cachedSerdoc !== undefined;

		if (targetRevision === seq && hasCache) return; // Already loaded and cached

		// Only show loading state if we don't have cache
		if (!hasCache) {
			setDiffLoading(true);
		}

		try {
			// Fetch the revision at this seq using the documents service
			let newSerdoc: any = cachedSerdoc;
			if (!newSerdoc) {
				newSerdoc = await api.documents.getRevisionContent(
					props.channel.id,
					`${props.channel.id}@${seq}`,
				);
				if (!newSerdoc) return;
			}

			// Fetch the previous revision for diff
			const prevSeq = Math.max(0, seq - 1);
			let oldSerdoc: any = null;
			if (prevSeq > 0) {
				const prevRevisionId = `${props.channel.id}@${prevSeq}`;
				oldSerdoc = api.documents.revisionCache.get(prevRevisionId) ?? null;
				if (!oldSerdoc) {
					oldSerdoc = await api.documents.getRevisionContent(
						props.channel.id,
						prevRevisionId,
					);
					if (!oldSerdoc) return;
				}
			}

			// Compute diff marks BEFORE setting state (state change resets plugin)
			const marks = computeDiffMarks(oldSerdoc ?? {}, newSerdoc);

			// Convert serdoc to HTML for the editor
			const newHtml = serdocToHtml(newSerdoc);

			// Use the editor's createReadonlyState method (no Yjs sync)
			const readonlyState = ed.createReadonlyStateFromHtml(newHtml);
			ed.setState(readonlyState);

			// Set diff marks AFTER setting state (otherwise they get lost!)
			ed.setDiffMarks(marks);

			if (isPreview) {
				setPreviewRevision(seq);
			} else {
				setCurrentRevision(seq);
			}
		} catch (e) {
			console.error("Failed to load revision:", e);
		} finally {
			if (!hasCache) {
				setDiffLoading(false);
			}
		}
	};

	onCleanup(() => {
		props.setEditor(null);
	});

	// Restore version dropdown state
	const [restoreMenuOpen, setRestoreMenuOpen] = createSignal(false);
	const [restoreBtn, setRestoreBtn] = createSignal<HTMLElement>();
	const [restoreMenu, setRestoreMenu] = createSignal<HTMLElement>();
	const restorePos = useFloating(restoreBtn, restoreMenu, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-end",
		middleware: [offset(4), flip(), shift()],
	});

	// Close restore menu on click outside
	onMount(() => {
		const close = () => setRestoreMenuOpen(false);
		window.addEventListener("click", close);
		onCleanup(() => window.removeEventListener("click", close));
	});

	// Handle restoring a historical version
	const handleRestoreVersion = async (mode: "current" | "new") => {
		const seq = props.selectedSeq;
		if (seq === null) return;

		setRestoreMenuOpen(false);

		try {
			if (mode === "new") {
				// Fork a new branch from the selected revision
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
				// TODO: Restore to current branch (needs API endpoint)
				console.log("Restore to current branch @seq:", seq);
			}
		} catch (e) {
			console.error("Failed to restore version:", e);
		}
	};

	return (
		<>
			<Show when={mode() === "diff_readonly" && currentChangeset()}>
				{(changeset) => (
					<div class="diff-view-inline">
						<div class="diff-view-header">
							<span class="diff-view-title">
								Viewing revision from{" "}
								<Time date={new Date(changeset().start_time)} />
							</span>
							<div class="diff-view-stats">
								<span class="diff-stat-added">
									+{changeset().stat_added}
								</span>
								<span class="diff-stat-removed">
									−{changeset().stat_removed}
								</span>
							</div>
							<button
								ref={setRestoreBtn}
								class="diff-view-close"
								onClick={(e) => {
									e.stopPropagation();
									setRestoreMenuOpen(!restoreMenuOpen());
								}}
							>
								Restore ▼
							</button>
							<button
								class="diff-view-close"
								onClick={() => props.onSelectChangeset(null)}
							>
								Back to current version
							</button>
						</div>
					</div>
				)}
			</Show>
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
			<main>
				{(() => {
					const ed = editor();
					if (!ed) return null;
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
		</>
	);
};

// Helper function to compute DiffMark[] from old and new serdocs
function computeDiffMarks(oldSerdoc: any, newSerdoc: any): DiffMark[] {
	const oldDoc = serdocToProseMirrorDoc(oldSerdoc);
	const newDoc = serdocToProseMirrorDoc(newSerdoc);

	// If we can't parse either document, return empty marks
	if (!oldDoc || !newDoc) {
		return [];
	}

	// Extract text content from PM documents (this preserves exact text structure)
	const oldText = oldDoc.textContent;
	const newText = newDoc.textContent;

	// Build position maps: string index -> ProseMirror position
	const oldPosMap = buildPositionMap(oldDoc);
	const newPosMap = buildPositionMap(newDoc);

	// Run diff on text content
	const changes = diffWords(oldText, newText);

	const marks: DiffMark[] = [];
	let oldStringPos = 0;
	let newStringPos = 0;

	for (const change of changes) {
		const len = change.value.length;

		if (change.added) {
			// Map string positions to ProseMirror positions in the new document
			const from = mapStringToPMPosition(newPosMap, newStringPos);
			const to = mapStringToPMPosition(newPosMap, newStringPos + len);
			marks.push({
				type: "insertion",
				from,
				to,
			});
			newStringPos += len;
		} else if (change.removed) {
			// Map string positions to ProseMirror positions in the old document
			// Deletions are shown at the current position in the new document
			const pos = mapStringToPMPosition(newPosMap, newStringPos);
			marks.push({
				type: "deletion",
				pos,
				text: change.value,
			});
			oldStringPos += len;
		} else {
			// Unchanged text
			oldStringPos += len;
			newStringPos += len;
		}
	}

	return marks;
}

// Convert serdoc to ProseMirror document
function serdocToProseMirrorDoc(serdoc: any) {
	try {
		const doc = serdoc?.data ?? serdoc;

		// Handle different serdoc formats
		if (!doc) {
			return null;
		}

		// Format 1: { root: { blocks: [...] } }
		if (doc?.root?.blocks) {
			const htmlParts: string[] = [];
			for (const block of doc.root.blocks) {
				if (block.Markdown?.content) {
					htmlParts.push(block.Markdown.content);
					htmlParts.push("\n\n");
				}
			}
			if (htmlParts.length === 0) {
				// Empty document - create empty paragraph
				htmlParts.push("<p></p>");
			}
			const div = document.createElement("div");
			div.innerHTML = htmlParts.join("");
			return DOMParser.fromSchema(schema).parse(div);
		}

		// Format 2: Already HTML
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

// Convert serdoc to HTML string
function serdocToHtml(serdoc: any): string {
	try {
		const doc = serdoc?.data ?? serdoc;
		if (!doc) {
			return "";
		}

		// Format 1: { root: { blocks: [...] } }
		if (doc?.root?.blocks) {
			const htmlParts: string[] = [];
			for (const block of doc.root.blocks) {
				if (block.Markdown?.content) {
					htmlParts.push(block.Markdown.content);
				}
			}
			if (htmlParts.length === 0) {
				return "";
			}
			return htmlParts.join("\n\n");
		}

		// Format 2: Already HTML
		if (typeof doc === "string") {
			return doc;
		}

		return "";
	} catch (e) {
		console.error("Failed to convert serdoc to HTML:", e, serdoc);
		return "";
	}
}

// Build a map from string index to ProseMirror position
// Returns array where index = string position, value = PM position
// This walks the PM document and maps each text character to its PM position
function buildPositionMap(doc: any): number[] {
	if (!doc) {
		return [];
	}

	const result: number[] = [];

	doc.descendants((node: any, pos: number) => {
		if (node.isText) {
			// Map each character in the text node to its PM position
			for (let i = 0; i < node.text!.length; i++) {
				result.push(pos + 1 + i);
			}
		}
		return !node.isLeaf;
	});

	return result;
}

// Map a string position to ProseMirror position using the position map
function mapStringToPMPosition(posMap: number[], stringPos: number): number {
	if (stringPos >= posMap.length) {
		return posMap.length > 0 ? posMap[posMap.length - 1] + 1 : 0;
	}
	return posMap[stringPos] ?? 0;
}

// Convert Y.XmlFragment to plain text
function yXmlFragmentToText(xmlFragment: Y.XmlFragment): string {
	const textParts: string[] = [];
	xmlFragment.forEach((item) => {
		if (item instanceof Y.XmlText) {
			textParts.push(item.toString());
		} else if (item instanceof Y.XmlElement) {
			textParts.push("\n");
			textParts.push(yXmlFragmentToText(item as unknown as Y.XmlFragment));
		}
	});
	return textParts.join("");
}
