import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { TextSelection } from "prosemirror-state";
import { useFloating } from "solid-floating-ui";
import {
	createMemo,
	createSignal,
	For,
	type JSX,
	onCleanup,
	onMount,
	type ParentProps,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import type { DocumentBranch } from "ts-sdk";
import { useApi, useDocumentBranches, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import { Icon } from "@/atoms/Icon";
import { timeAgo } from "@/atoms/Time";
import { useChannel, useModals } from "@/contexts/mod";
import type { ChannelT } from "@/types";
import {
	icBranch,
	icBranchDefault,
	icBranchFork,
	icBranchNew,
	icBranchPrivate,
	icDelete,
	icFormatBold,
	icFormatCode,
	icFormatItalic,
	icFormatStrikethrough,
	icFormatUrl,
	icMembers,
	icMergeCherrypick,
	icMergeFull,
	icRename,
	icSync,
} from "@/utils/icons";
import { useDocument } from "./context";
import { exportAsHtml, exportAsMarkdown, generateFilename } from "./export";

export type DocumentHeaderProps = {
	channel: ChannelT;
};

export const DocumentHeader = (props: DocumentHeaderProps) => {
	const ctx = useCtx();
	const doc = useDocument();
	const [, modalCtl] = useModals();
	const [ch, setCh] = useChannel()!;
	const [active, setActive] = createSignal<
		"branches" | "merge" | "export" | "insert" | null
	>(null);

	const branches = useDocumentBranches();
	const users = useUsers();
	const api = useApi();
	const [filterText, setFilterText] = createSignal("");

	const toggleMembers = () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				showMembers: !(c.frontend.showMembers ?? true),
			},
		});
	};

	// Paginated list of branches for this channel
	const branchList = branches.useList(() => props.channel.id);

	// Filtered branches based on search text
	const filteredBranches = createMemo(() => {
		const list = branchList();
		if (!list) return [];
		const filter = filterText().toLowerCase();
		return list.state.ids
			.map((id) => branches.cache.get(id))
			.filter((b): b is DocumentBranch => b !== undefined)
			.filter((b) => !b.default)
			.filter(
				(b) => !filter || (b.name?.toLowerCase().includes(filter) ?? false),
			);
	});

	const currentBranch = createMemo(() => {
		return branches.cache.get(doc.branchId);
	});

	// Default branch: the one with default === true
	const defaultBranch = createMemo(() => {
		const list = branchList();
		if (!list) return undefined;
		return list.state.ids
			.map((id) => branches.cache.get(id))
			.find((b): b is DocumentBranch => b?.default === true);
	});

	// Resolve the default branch ID (fall back to channel id if not yet loaded)
	const defaultBranchId = createMemo(() => {
		const db = defaultBranch();
		return db?.id ?? props.channel.id;
	});

	const toggleHistory = () => {
		setCh("history_view", !ch.history_view);
	};

	const handleNewBranch = async () => {
		try {
			const parent = defaultBranchId();
			const branch = await api.documentBranches.fork(props.channel.id, parent, {
				name: "untitled branch",
				private: false,
			});
			update("branchId", branch.id);
			setActive(null);
		} catch (e) {
			console.error("Failed to create new branch:", e);
		}
	};

	const handleNewPrivateBranch = async () => {
		try {
			const parent = defaultBranchId();
			const branch = await api.documentBranches.fork(props.channel.id, parent, {
				name: "untitled private branch",
				private: true,
			});
			update("branchId", branch.id);
			setActive(null);
		} catch (e) {
			console.error("Failed to create new private branch:", e);
		}
	};

	const applyFormat = (wrap: string) => {
		const view = props.editor?.().view;
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
			modalCtl.open({ type: "link", editor: props.editor().view });
		}
	};

	const handleExportMarkdown = () => {
		const editor = props.editor();
		const view = editor?.view;
		if (!view) return;
		const filename = generateFilename(props.channel.name, "md");
		exportAsMarkdown(view, filename);
		setActive(null);
	};

	const handleExportHtml = () => {
		const editor = props.editor();
		const view = editor?.view;
		if (!view) return;
		const filename = generateFilename(props.channel.name, "html");
		exportAsHtml(view, filename, props.channel.name);
		setActive(null);
	};

	const handleMergeFull = async () => {
		try {
			await api.documents.merge(props.channel.id, doc.branchId);
			update("branchId", defaultBranchId());
			setActive(null);
		} catch (e) {
			console.error("Failed to merge branch:", e);
		}
	};

	const handleRenameBranch = () => {
		const branch = currentBranch();
		if (!branch) return;

		modalCtl.prompt("rename branch", (newName) => {
			if (newName && newName !== branch.name) {
				api.documentBranches.update(props.channel.id, branch.id, {
					name: newName,
				});
			}
		});
		setActive(null);
	};

	const handleDeleteBranch = () => {
		const branch = currentBranch();
		if (!branch) return;

		// TODO: warn if there are changes between the branch and its parent
		modalCtl.confirm(
			`Are you sure you want to delete the branch "${
				branch.name || "unnamed"
			}"?`,
			async (confirmed) => {
				if (confirmed) {
					try {
						await api.documentBranches.close(props.channel.id, branch.id);
						update("branchId", defaultBranchId());
					} catch (e) {
						console.error("Failed to delete branch:", e);
					}
				}
			},
		);
		setActive(null);
	};

	const handleSyncBranch = async () => {
		try {
			await api.documentBranches.sync(props.channel.id, doc.branchId);
			setActive(null);
		} catch (e) {
			console.error("Failed to sync branch:", e);
		}
	};

	const suggestedRenames = [
		"fix/typos",
		"feature/content-update",
		"docs/clarification",
		"refactor/intro",
	];

	// TODO: extract buttons into component
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
		<header class="document-header">
			<div class="menu-group">
				<button
					type="button"
					class="button"
					ref={setBranchBtn}
					onClick={(e) => {
						e.stopPropagation();
						setActive(active() === "branches" ? null : "branches");
					}}
					classList={{ active: active() === "branches" }}
				>
					{currentBranch()?.name ||
						(currentBranch()?.default ? "main" : "unnamed")}
				</button>
				<Show when={currentBranch()?.parent_id}>
					<button
						type="button"
						class="button"
						ref={setMergeBtn}
						onClick={(e) => {
							e.stopPropagation();
							setActive(active() === "merge" ? null : "merge");
						}}
						classList={{ active: active() === "merge" }}
					>
						branch
					</button>
				</Show>
				<button
					type="button"
					class="button"
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
					type="button"
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("**");
					}}
				>
					<Icon src={icFormatBold} />
				</button>
				<button
					type="button"
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("*");
					}}
				>
					<Icon src={icFormatItalic} />
				</button>
				<button
					type="button"
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("~~");
					}}
				>
					<Icon src={icFormatStrikethrough} />
				</button>
				<button
					type="button"
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						applyFormat("`");
					}}
				>
					<Icon src={icFormatCode} />
				</button>
				<button
					type="button"
					class="icon-button"
					onClick={(e) => {
						e.stopPropagation();
						openLinkModal();
					}}
				>
					<Icon src={icFormatUrl} />
				</button>
				<button
					type="button"
					class="button"
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
					type="button"
					class="button"
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

			<div style="flex:1"></div>
			<menu class="right">
				<button type="button" onClick={toggleMembers} title="Show members">
					<Icon src={icMembers} />
				</button>
			</menu>

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
							onInput={(e) => setFilterText(e.currentTarget.value)}
						/>
						<ul>
							{/* Default branch */}
							<li
								class="default"
								classList={{ selected: doc.branchId === defaultBranchId() }}
								onClick={() => {
									update("branchId", defaultBranchId());
									setActive(null);
								}}
							>
								<button type="button" class="button">
									<Icon src={icBranchDefault} />
									<div class="info">
										<div>default</div>
										<div class="dim">the main/master/default branch</div>
									</div>
								</button>
							</li>
							<For each={filteredBranches()}>
								{(branch) => {
									const creator = users.cache.get(branch.creator_id);
									// TODO: use data attributes
									const stateColor =
										branch.state === "Active"
											? "color: $color-green"
											: branch.state === "Closed"
												? "color: $color-warn"
												: "color: $color-fg-500";
									return (
										<li classList={{ private: branch.private }}>
											<button
												type="button"
												class="button"
												onClick={() => {
													update("branchId", branch.id);
													setActive(null);
												}}
												classList={{ selected: doc.branchId === branch.id }}
											>
												<Icon
													src={branch.private ? icBranchPrivate : icBranch}
												/>
												<div class="info">
													<div>
														{branch.name || "unnamed"}
														{branch.private && (
															<span class="dim"> (private)</span>
														)}
													</div>
													<div class="dim" style={stateColor}>
														{branch.state.toLowerCase()}
														{creator && (
															<>
																{" "}
																· created by{" "}
																<b>
																	{creator.relationship.petname || creator.name}
																</b>
															</>
														)}
														{branch.created_at && (
															<> · {timeAgo(new Date(branch.created_at))}</>
														)}
													</div>
												</div>
											</button>
										</li>
									);
								}}
							</For>
							<li class="separator"></li>
							<li class="new">
								<button type="button" class="button" onClick={handleNewBranch}>
									<Icon src={icBranchNew} />
									<div class="info">
										<div>new</div>
										<div class="dim">create a new branch</div>
									</div>
								</button>
							</li>
							<li class="new">
								<button type="button" class="button">
									<Icon src={icBranchFork} />
									<div class="info">
										<div>new from changes</div>
										<div class="dim">
											create a new branch from existing changes
										</div>
									</div>
								</button>
							</li>
							<li class="new">
								<button
									type="button"
									class="button"
									onClick={handleNewPrivateBranch}
								>
									<Icon src={icBranchFork} />
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
						class="branch-action-menu document-menu"
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
								<button
									type="button"
									class="button"
									onClick={handleRenameBranch}
								>
									<Icon src={icRename} />
									<div class="info">
										<div>rename</div>
										<div class="dim">change the name of this branch</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={handleSyncBranch}>
									<Icon src={icSync} />
									<div class="info">
										<div>sync</div>
										<div class="dim">pull changes from parent</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={handleMergeFull}>
									<Icon src={icMergeFull} />
									<div class="info">
										<div>merge</div>
										<div class="dim">fully merge all changes</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button">
									<Icon src={icMergeCherrypick} />
									<div class="info">
										<div>cherry pick</div>
										<div class="dim">merge specific changes</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={handleDeleteBranch}
								>
									<Icon src={icDelete} />
									<div class="info">
										<div style="color: $color-warn">delete</div>
										<div class="dim">permanently remove this branch</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li
								class="header"
								style="padding: 4px 12px; font-weight: bold; font-size: 0.8em; opacity: 0.7"
							>
								suggested renames
							</li>
							<For each={suggestedRenames}>
								{(name) => (
									<li>
										<button
											type="button"
											class="button"
											onClick={() => {
												const branch = currentBranch();
												if (branch) {
													api.documentBranches.update(
														props.channel.id,
														branch.id,
														{
															name,
														},
													);
												}
												setActive(null);
											}}
										>
											<div class="info">
												<div>{name}</div>
											</div>
										</button>
									</li>
								)}
							</For>
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
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
									<div class="info">
										<div>{false ? "open in new tab" : "publish document"}</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li>
								<button type="button" class="button" onClick={handleExportHtml}>
									<div class="info">
										<div>download as html</div>
										<div class="dim">single file .html file</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={handleExportMarkdown}
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
								<button
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
									<div class="info">
										<div>media</div>
										<div class="dim">insert images, videos, and audio</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
									<div class="info">
										<div>table</div>
										<div class="dim">insert tables with rows and columns</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
									<div class="info">
										<div>code</div>
										<div class="dim">
											insert code blocks with syntax highlighting
										</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
									<div class="info">
										<div>symbols</div>
										<div class="dim">insert special characters and symbols</div>
									</div>
								</button>
							</li>
							<li>
								<button
									type="button"
									class="button"
									onClick={() => setActive(null)}
								>
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

type MenubarItemProps = {
	button: JSX.Element;
};

const MenubarItem = (props: ParentProps<MenubarItemProps>) => {
	const [open, setOpen] = createSignal(false);

	// TODO: close menu when clicking outside

	return (
		<>
			<button
				type="button"
				class="button"
				// ref={setExportBtn}
				onClick={[setOpen, true]}
				classList={{ active: open() }}
			>
				{props.button}
			</button>
			<Show when={open()}>
				<Portal>
					<menu
						// TODO: implement this
						class="document-menu"
						// ref={setExportMenu}
						// style={{
						// 	position: exportPos.strategy,
						// 	top: `${exportPos.y ?? 0}px`,
						// 	left: `${exportPos.x ?? 0}px`,
						// 	"z-index": 100,
						// }}
						onClick={(e) => e.stopPropagation()}
					>
						{props.children}
					</menu>
				</Portal>
			</Show>
		</>
	);
};
