import type { Channel } from "sdk";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { createEditor } from "./DocumentEditor.tsx";
import icBranchDefault from "./assets/edit.png";
import icBranchPrivate from "./assets/edit.png";
import icBranchNew from "./assets/edit.png";
import icBranchFork from "./assets/edit.png";
import icBranch from "./assets/edit.png";
import icMergeFull from "./assets/edit.png";
import icMergeCherrypick from "./assets/edit.png";
import { useDocument } from "./contexts/document.tsx";

type DocumentProps = {
	channel: Channel;
};

export const Document = (props: DocumentProps) => {
	const [branchId, setBranchId] = createSignal(props.channel.id);
	// setup ydoc here, pass to DocumentMain?

	return (
		<div class="document">
			<DocumentHeader channel={props.channel} />
			<DocumentMain channel={props.channel} />
		</div>
	);
};

const DocumentHeader = (props: DocumentProps) => {
	const [doc, update] = useDocument();
	const [active, setActive] = createSignal<"branches" | "merge" | "export" | null>(null);

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

	onMount(() => {
		const close = () => setActive(null);
		window.addEventListener("click", close);
		onCleanup(() => window.removeEventListener("click", close));
	});

	// top: title, topic(?), notifications, members, search
	// bottom: branches (merge, diff), edit, format, insert, view, tools
	return (
		<header>
			<div class="fake-dropdowns">
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
						class="branch-menu"
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
							autofocus
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
						class="merge-menu"
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
						class="export-menu"
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
								<button onClick={() => {
									// TODO: publishing documents
									setActive(null);
								}}>
									<div class="info">
										<div>{false ? "open in new tab" : "publish document"}</div>
									</div>
								</button>
							</li>
							<li class="separator"></li>
							<li>
								<button onClick={() => {
									// TODO: download as html
									setActive(null);
								}}>
									<div class="info">
										<div>download as html</div>
										<div class="dim">single file .mhtml file</div>
									</div>
								</button>
							</li>
							<li>
								<button onClick={() => {
									// TODO: download as markdown (how do i handle media?)
									setActive(null);
								}}>
									<div class="info">
										<div>download as markdown</div>
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

const DocumentMain = (props: DocumentProps) => {
	const editor = createEditor({}, props.channel.id, props.channel.id);

	return (
		<main>
			<editor.View
				onSubmit={() => false}
				channelId={props.channel.id}
			/>
		</main>
	);
};

export const Wiki = (props: DocumentProps) => {
	// maybe copy forum channels here
	return "todo";
};
