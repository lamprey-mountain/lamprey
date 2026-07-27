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
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import type { DocumentBranch } from "ts-sdk";
import { useApi, useDocumentBranches, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import { Icon } from "@/atoms/Icon";
import { Search } from "@/atoms/Search";
import { Time, timeAgo } from "@/atoms/Time";
import { createTooltip } from "@/atoms/Tooltip";
import { useChannel, useModals } from "@/contexts/mod";
import { flags } from "@/lib/flags";
import type { ChannelT } from "@/types";
import {
	icBranch,
	icBranchDefault,
	icBranchFork,
	icBranchNew,
	icBranchPrivate,
	icComments,
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
	icTableOfContents,
	icThread,
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
		return branches.cache.get(doc.branchId());
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
	// NOTE: default branch id will be different from the channel id soon
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
			doc.setBranchId(branch.id);
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
			doc.setBranchId(branch.id);
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
	};

	const handleExportHtml = () => {
		const editor = props.editor();
		const view = editor?.view;
		if (!view) return;
		const filename = generateFilename(props.channel.name, "html");
		exportAsHtml(view, filename, props.channel.name);
	};

	const handleMergeFull = async () => {
		try {
			await api.documents.merge(props.channel.id, doc.branchId());
			doc.setBranchId(defaultBranchId());
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
						doc.setBranchId(defaultBranchId());
					} catch (e) {
						console.error("Failed to delete branch:", e);
					}
				}
			},
		);
	};

	const handleSyncBranch = async () => {
		try {
			await api.documentBranches.sync(props.channel.id, doc.branchId());
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

	const showMembers = () =>
		flags.has("room_member_list") &&
		ctx.preferences().frontend.showMembers !== false;

	// TODO: make threadTooltip and tocTooltip also say "Show"/"Hide" instead of "Toggle"
	// TODO: port membersTooltip to ChatHeader
	const tocTooltip = createTooltip({ tip: () => "Toggle table of contents" });
	const commentsTooltip = createTooltip({ tip: () => "View comments" });
	const threadTooltip = createTooltip({ tip: () => "Toggle chat" });
	const membersTooltip = createTooltip({
		tip: () => (showMembers() ? "Hide members" : "Show members"),
	});

	return (
		<header class="document-header chat-header">
			<div class="menu-group">
				<MenubarItem
					button={
						currentBranch()?.name ||
						(currentBranch()?.default ? "main" : "unnamed")
					}
				>
					{(close) => (
						<>
							<Search
								placeholder="filter branches..."
								onInput={(input) => setFilterText(input)}
								ref={(el) => queueMicrotask(() => el.focus())}
							/>
							<ul>
								{/* Default branch */}
								<li
									class="default"
									classList={{ selected: doc.branchId() === defaultBranchId() }}
									onClick={() => {
										doc.setBranchId(defaultBranchId());
										close();
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
														doc.setBranchId(branch.id);
														close();
													}}
													classList={{ selected: doc.branchId() === branch.id }}
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
																		{creator.relationship.petname ||
																			creator.name}
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
									<button
										type="button"
										class="button"
										onClick={() => {
											handleNewBranch();
											close();
										}}
									>
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
										onClick={() => {
											handleNewPrivateBranch();
											close();
										}}
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
						</>
					)}
				</MenubarItem>
				<Show when={currentBranch()?.parent_id}>
					<MenubarItem button="branch">
						{(close) => (
							<>
								<ul>
									<li>
										<button
											type="button"
											class="button"
											onClick={() => {
												handleRenameBranch();
												close();
											}}
										>
											<Icon src={icRename} />
											<div class="info">
												<div>rename</div>
												<div class="dim">change the name of this branch</div>
											</div>
										</button>
									</li>
									<li>
										<button
											type="button"
											class="button"
											onClick={() => {
												handleSyncBranch();
												close();
											}}
										>
											<Icon src={icSync} />
											<div class="info">
												<div>sync</div>
												<div class="dim">pull changes from parent</div>
											</div>
										</button>
									</li>
									<li>
										<button
											type="button"
											class="button"
											onClick={() => {
												handleMergeFull();
												close();
											}}
										>
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
											onClick={() => {
												handleDeleteBranch();
												close();
											}}
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
														close();
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
							</>
						)}
					</MenubarItem>
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
				<MenubarItem button="insert">
					{(close) => (
						<ul>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>media</div>
										<div class="dim">insert images, videos, and audio</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>table</div>
										<div class="dim">insert tables with rows and columns</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>code</div>
										<div class="dim">
											insert code blocks with syntax highlighting
										</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>symbols</div>
										<div class="dim">insert special characters and symbols</div>
									</div>
								</button>
							</li>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>time</div>
										<div class="dim">insert current date and time</div>
									</div>
								</button>
							</li>
						</ul>
					)}
				</MenubarItem>
			</div>
			<div class="menu-group">
				<MenubarItem button="export">
					{(close) => (
						<ul>
							<li>
								<button type="button" class="button" onClick={close}>
									<div class="info">
										<div>{false ? "open in new tab" : "publish document"}</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li>
								<button
									type="button"
									class="button"
									onClick={() => {
										handleExportHtml();
										close();
									}}
								>
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
									onClick={() => {
										handleExportMarkdown();
										close();
									}}
								>
									<div class="info">
										<div>download as markdown</div>
									</div>
								</button>
							</li>
						</ul>
					)}
				</MenubarItem>
			</div>

			<div style="flex:1"></div>
			<menu class="menu right">
				<button
					type="button"
					onClick={() => doc.setTocOpen(!doc.tocOpen())}
					ref={tocTooltip.content}
				>
					<Icon src={icTableOfContents} />
				</button>
				<button
					type="button"
					onClick={() => {
						// TODO: copy ChatHeader ctx.setThreadsView
					}}
					ref={commentsTooltip.content}
				>
					<Icon src={icComments} />
				</button>
				<button
					type="button"
					onClick={() => {
						/* TODO */
					}}
					ref={threadTooltip.content}
				>
					<Icon src={icThread} />
				</button>
				<button
					type="button"
					onClick={toggleMembers}
					ref={membersTooltip.content}
				>
					<Icon src={icMembers} />
				</button>
			</menu>
		</header>
	);
};

type MenubarItemProps = {
	button: JSX.Element;
	children: (close: () => void) => JSX.Element;
};

const MenubarItem = (props: MenubarItemProps) => {
	const [buttonRef, setButtonRef] = createSignal<HTMLElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [open, setOpen] = createSignal(false);

	const pos = useFloating(buttonRef, menuRef, {
		whileElementsMounted: autoUpdate,
		placement: "bottom-start",
		middleware: [offset(4), flip(), shift()],
	});

	onMount(() => {
		const close = (e: MouseEvent) => {
			const target = e.target as HTMLElement;
			const m = menuRef();
			const b = buttonRef();
			if (m?.contains(target) || b?.contains(target)) return;
			setOpen(false);
		};

		window.addEventListener("click", close);
		onCleanup(() => window.removeEventListener("click", close));
	});

	return (
		<>
			<button
				type="button"
				class="button"
				ref={setButtonRef}
				onClick={() => setOpen(!open())}
				classList={{ active: open() }}
			>
				{props.button}
			</button>
			<Show when={open()}>
				<Portal>
					<menu
						class="document-menu"
						ref={setMenuRef}
						style={{
							position: pos.strategy,
							top: `${pos.y ?? 0}px`,
							left: `${pos.x ?? 0}px`,
							"z-index": 100,
						}}
					>
						{props.children(() => setOpen(false))}
					</menu>
				</Portal>
			</Show>
		</>
	);
};
