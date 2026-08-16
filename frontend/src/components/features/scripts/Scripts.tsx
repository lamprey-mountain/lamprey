import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { useNavigate } from "@solidjs/router";
import fuzzysort from "fuzzysort";
import { type Channel, createUpload, type Media, type Script } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useApi } from "@/api";
import { Search } from "@/atoms/Search";
import { createPanes } from "@/components/panes/context";
import { useChannel } from "@/contexts/channel";
import { ChatHeader } from "../chat/ChatHeader";
import { createScriptContext, ScriptContext } from "./context";
import { RunLogs, ScriptCode, ScriptInputs, ScriptPreview } from "./Panes";

// in channel nav: show current script like a thread

export const Scripts = (props: { channel: Channel }) => {
	const api = useApi();
	const s = createScriptContext(props.channel.id);

	const [scriptsResource] = createResource(
		() => props.channel.id,
		(id) => api.scripts.list(id),
	);

	const [search, setSearch] = createSignal("");
	const navigate = useNavigate();

	const filteredScripts = () => {
		const items = scriptsResource()?.items ?? [];
		const query = search();
		if (!query) return items;
		const results = fuzzysort.go(query, items, {
			key: "name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const openScript = (script: Script) => {
		navigate(`/channel/${props.channel.id}/script/${script.id}`);
	};

	const panes = createPanes({
		types: {
			script_code: (props) => (
				<ScriptCode pane={props.pane} setHeaderExtra={props.setHeaderExtra} />
			),
			script_inputs: (props) => <ScriptInputs pane={props.pane} />,
			script_preview: () => <ScriptPreview />,
			run_logs: (props) => <RunLogs pane={props.pane} />,
		},
	});

	// Auto-open script when script_id is set in channel state
	const [ch, updateCh] = useChannel();
	createEffect(() => {
		const scriptId = ch.script_id;
		if (!scriptId) return;

		const items = scriptsResource()?.items ?? [];
		const script = items.find((s) => s.id === scriptId);
		if (!script) return;

		panes.closeAll();
		panes.create({
			id: 0,
			type: "split_horizontal",
		});
		panes.create({
			id: 1,
			parentId: 0,
			type: "leaf",
			data: {
				type: "script_code",
				script_id: script.id,
			},
		});
		panes.create({
			id: 2,
			parentId: 0,
			type: "leaf",
			data: {
				type: "script_inputs",
				script_id: script.id,
			},
		});
		api.scriptLogs.subscribe(props.channel.id, script.id);
		updateCh("script_id", undefined);
	});

	const [createOpen, setCreateOpen] = createSignal(false);
	const [referenceEl, setReferenceEl] = createSignal<HTMLElement>();
	const [floatingEl, setFloatingEl] = createSignal<HTMLElement>();
	const position = useFloating(referenceEl, floatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(5), flip(), shift()],
		placement: "bottom-end",
	});

	let scriptUploadRef!: HTMLInputElement;

	const onCreateDocument = async () => {
		const redex = await api.scripts.create(props.channel.id, {
			format: "Javascript",
			location: { type: "Document" },
		});
		updateCh("script_id", redex.id);
	};

	const onCreateUpload = async () => {
		scriptUploadRef.click();
	};

	const onUpload = async () => {
		// TODO: accept multiple files
		const file = scriptUploadRef.files?.[0];
		if (!file) return;
		createUpload({
			client: api.client,
			file,
			onProgress(_progress: number) {
				// TODO: progress indicator
			},
			// TODO(future): pause/resume support
			onPause() {},
			onResume() {},
			onFail(_error: Error) {
				// TODO: error handling
			},
			async onComplete(media: Media) {
				const redex = await api.scripts.create(props.channel.id, {
					format: file.name.endsWith(".wasm") ? "Webassembly" : "Javascript",
					location: { type: "Hosted", media_id: media.id },
				});
				updateCh("script_id", redex.id);
			},
		});
	};

	return (
		<ScriptContext.Provider value={s}>
			<ChatHeader channel={props.channel} />
			<div class="scripts" style="grid-area:main">
				<panes.Render
					placeholder={
						<div class="script-list">
							<header class="scripts-header">
								<Search
									placeholder="Search scripts..."
									value={search}
									onInput={setSearch}
								/>
								<div class="script-create-container">
									<button
										type="button"
										class="button primary"
										ref={setReferenceEl}
										onClick={() => setCreateOpen(!createOpen())}
										classList={{ open: createOpen() }}
									>
										create
									</button>
									<input
										type="file"
										style="display:none"
										ref={scriptUploadRef}
										onInput={onUpload}
										accept=".js,.wasm,text/javascript,application/wasm"
									/>
									<Portal>
										<Show when={createOpen()}>
											<div
												ref={setFloatingEl}
												class="script-create-menu"
												style={{
													position: position.strategy,
													top: `${position.y ?? 0}px`,
													left: `${position.x ?? 0}px`,
													"z-index": 1000,
												}}
											>
												{/* TODO: icons, descriptions */}
												<menu class="inner">
													<button
														type="button"
														class="button"
														onClick={onCreateDocument}
													>
														document
													</button>
													<button
														type="button"
														class="button"
														onClick={onCreateUpload}
													>
														upload
													</button>
												</menu>
											</div>
										</Show>
									</Portal>
								</div>
							</header>
							<ul>
								<For each={filteredScripts()}>
									{(script) => (
										<li>
											<button type="button" onClick={() => openScript(script)}>
												<span class="name">
													{script.latest_version.metadata.name}
												</span>
												<Show when={script.latest_version.metadata.description}>
													{(d) => <span class="description">{d()}</span>}
												</Show>
											</button>
										</li>
									)}
								</For>
							</ul>
						</div>
					}
				/>
			</div>
		</ScriptContext.Provider>
	);
};
