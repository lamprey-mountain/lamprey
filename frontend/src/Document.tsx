import type { Channel } from "sdk";
import { createSignal, Show } from "solid-js";
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

	// top: title, topic(?), notifications, members, search
	// bottom: branches (merge, diff), edit, format, insert, view, tools
	return (
		<header>
			<div class="fake-dropdowns">
				<button>branches</button>
				<Show when={true}>
					<button>merge</button>
				</Show>
			</div>
			<Show when={true}>
				<menu class="branch-menu">
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
							onClick={() => update("branchId", props.channel.id)}
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
					</ul>
				</menu>
			</Show>
			<br />
			<Show when={true}>
				<menu class="merge-menu">
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
			</Show>
			<br />
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
