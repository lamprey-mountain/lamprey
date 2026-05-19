import { useNavigate } from "@solidjs/router";
import fuzzysort from "fuzzysort";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	type JSX,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import type { Channel, Script } from "ts-sdk";
import { useScriptLogs, useScriptRuns, useScripts } from "@/api";
import { PaneResizeHandle } from "@/atoms/Resizable";
import { Time } from "@/atoms/Time";
import { useChannel } from "@/contexts/channel";
import { getUrl } from "@/media/util";
import {
	createScriptContext,
	ScriptContext,
	type ScriptPaneChild,
	type ScriptPane as ScriptPaneT,
	useScript,
} from "./context";
import { LazyCodeEditor } from "./ScriptEditor";

// in channel nav: show current script like a thread

export const Scripts = (props: { channel: Channel }) => {
	const s = createScriptContext(props.channel.id);
	const scriptsService = useScripts();
	const logs = useScriptLogs();

	const [scriptsResource] = createResource(
		() => props.channel.id,
		(id) => scriptsService.list(id),
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

	// Auto-open script when script_id is set in channel state
	const ch = useChannel();
	createEffect(() => {
		const scriptId = ch[0].script_id;
		if (!scriptId) return;

		const items = scriptsResource()?.items ?? [];
		const script = items.find((s) => s.id === scriptId);
		if (!script) return;

		s.reset();
		s.createPane({
			id: 0,
			type: "split_horizontal",
		});
		s.createPane({
			id: 1,
			parentId: 0,
			type: "script_code",
			script_id: script.id,
		});
		s.createPane({
			id: 2,
			parentId: 0,
			type: "script_inputs",
			script_id: script.id,
		});
		logs.subscribe(props.channel.id, script.id);
		ch[1]("script_id", undefined);
	});

	return (
		<ScriptContext.Provider value={s}>
			<div class="scripts" style="grid-area:main">
				<Show
					when={s.root}
					fallback={
						<div class="script-list">
							<header>
								<input
									type="search"
									placeholder="Search scripts..."
									value={search()}
									onInput={(e) => setSearch(e.target.value)}
								/>
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
				>
					{(root) => <ScriptPaneRenderer pane={root()} />}
				</Show>
			</div>
		</ScriptContext.Provider>
	);
};

const ScriptPaneRenderer = (props: {
	pane: ScriptPaneChild;
	isHorizontal?: boolean;
}) => {
	const s = useScript();
	const size = () => props.pane.size;

	return (
		<div
			class="pane-container"
			style={{
				flex: size() ? `0 0 ${size()}px` : "1",
				"min-width": "0",
				"min-height": "0",
			}}
		>
			<Show
				when={
					props.pane.type === "split_horizontal" ||
					props.pane.type === "split_vertical"
				}
				fallback={<ScriptPane pane={props.pane} />}
			>
				<div
					class={
						props.pane.type === "split_horizontal"
							? "split-horizontal"
							: "split-vertical"
					}
				>
					<For
						each={
							(
								props.pane as Extract<
									ScriptPaneT,
									{ children: ScriptPaneChild[] }
								>
							).children
						}
					>
						{(child, index) => (
							<>
								<Show when={index() > 0}>
									<PaneResizeHandle
										isHorizontal={props.pane.type === "split_horizontal"}
										onResize={(sz) => {
											s.updatePaneSize(
												(
													props.pane as Extract<
														ScriptPaneT,
														{ children: ScriptPaneChild[] }
													>
												).children[index() - 1].id,
												sz,
											);
										}}
									/>
								</Show>
								<ScriptPaneRenderer
									pane={child}
									isHorizontal={props.pane.type === "split_horizontal"}
								/>
							</>
						)}
					</For>
				</div>
			</Show>
		</div>
	);
};

export const ScriptPane = (props: { pane: ScriptPaneT }) => {
	const s = useScript();
	const pane = props.pane;
	const navigate = useNavigate();
	const [headerExtra, setHeaderExtra] = createSignal<JSX.Element>(null);

	// TODO: use x icons for pane close button

	return (
		<div class="script-pane">
			<header>
				<nav>{pane.type.replace("script_", "").replace("_", " ")}</nav>
				<div class="title">Pane {pane.id}</div>
				{headerExtra()}
				<button
					type="button"
					class="close"
					onClick={() => {
						s.closePane(pane.id);
						if (!s.root) {
							navigate(`/channel/${s.channel_id}`);
						}
					}}
				>
					&times;
				</button>
			</header>
			<div class="pane-content">
				<Switch>
					<Match when={pane.type === "script_code"}>
						<ScriptCode
							pane={pane as Extract<ScriptPaneT, { type: "script_code" }>}
							setHeaderExtra={setHeaderExtra}
						/>
					</Match>
					<Match when={pane.type === "script_inputs"}>
						<ScriptInputs
							pane={pane as Extract<ScriptPaneT, { type: "script_inputs" }>}
						/>
					</Match>
					<Match when={pane.type === "script_preview"}>
						<ScriptPreview />
					</Match>
					<Match when={pane.type === "run_logs"}>
						<RunLogs
							pane={pane as Extract<ScriptPaneT, { type: "run_logs" }>}
						/>
					</Match>
				</Switch>
			</div>
		</div>
	);
};

export const ScriptCode = (props: {
	pane: Extract<ScriptPaneT, { type: "script_code" }>;
	setHeaderExtra: (el: JSX.Element) => void;
}) => {
	const scriptsService = useScripts();
	const script = () => scriptsService.get(props.pane.script_id);
	const [source, { mutate }] = createResource(
		() => {
			const loc = script()?.latest_version.location;
			if (loc?.type === "Hosted") return loc.media;
			return undefined;
		},
		(media) => {
			return fetch(getUrl(media)).then((r) => r.text());
		},
	);

	const [editedSource, setEditedSource] = createSignal<string>("");
	const [saving, setSaving] = createSignal(false);

	createEffect(() => {
		const s = source();
		if (s !== undefined) {
			setEditedSource(s);
		}
	});

	const hasEdits = () => {
		const orig = source() ?? "";
		const curr = editedSource();
		return curr !== "" && curr !== orig;
	};

	const handleSave = async () => {
		const scr = script();
		if (!scr) return;
		setSaving(true);
		try {
			await scriptsService.uploadAndSaveContent(
				scr.channel_id,
				scr.id,
				editedSource(),
			);
			mutate(editedSource());
		} catch (err) {
			console.error("Failed to save script:", err);
		} finally {
			setSaving(false);
		}
	};
	createEffect(() => {
		props.setHeaderExtra(
			<Show when={hasEdits()}>
				<button
					type="button"
					class="pane-header-save button primary"
					onClick={handleSave}
					disabled={saving()}
				>
					{saving() ? "Saving..." : "Save Edits"}
				</button>
			</Show>,
		);
	});

	onCleanup(() => {
		props.setHeaderExtra(null);
	});

	return (
		<div class="script-code-container">
			<div class="editor-wrapper">
				<LazyCodeEditor
					source={source()}
					loading={source.loading}
					onChange={setEditedSource}
				/>
			</div>
		</div>
	);
};

export const ScriptInputs = (props: {
	pane: Extract<ScriptPaneT, { type: "script_inputs" }>;
}) => {
	const s = useScript();
	const scriptsService = useScripts();
	const runsService = useScriptRuns();

	const [script] = createResource(
		() => `${s.channel_id}:${props.pane.script_id}`,
		(id) => scriptsService.fetch(id),
	);

	const [runs, { refetch: refetchRuns }] = createResource(
		() => props.pane.script_id,
		(id) => runsService.list(s.channel_id, id),
	);

	const trigger = async (inputId: string) => {
		await runsService.trigger(s.channel_id, props.pane.script_id, {
			async: true,
			exclusive: false,
			trigger_id: inputId,
		});
		refetchRuns();
	};

	// FIXME: input.type.type -> input.type

	return (
		<div class="script-inputs">
			<section>
				<h3>Inputs</h3>
				<div class="input-list">
					<For each={script()?.handlers}>
						{(input) => (
							<div class="script-input" data-input-type={input.type}>
								<Show when={input.type === "Manual"}>
									<button
										class="inner"
										type="button"
										onClick={() => trigger(input.id)}
									>
										<div>{input.label}</div>
										<div class="dim">{input.id}</div>
									</button>
								</Show>
								<Show when={input.type !== "Manual"}>
									<div class="inner">
										<div>{input.label}</div>
										<div class="dim">{input.id}</div>
									</div>
								</Show>
							</div>
						)}
					</For>
				</div>
			</section>
			<section>
				<h3>Recent Runs</h3>
				<ul class="run-list">
					<For each={runs()?.items}>
						{(run) => (
							<li>
								<div class="run-item">
									<div class="run-info">
										<span class="status" data-status={run.status}>
											{run.status}
										</span>
										<Time date={new Date(run.created_at)} />
									</div>
									<menu>
										<button
											type="button"
											onClick={() =>
												s.createPane({
													type: "run_logs",
													script_id: props.pane.script_id,
													run_id: run.id,
												})
											}
										>
											Logs
										</button>
									</menu>
								</div>
							</li>
						)}
					</For>
				</ul>
			</section>
		</div>
	);
};

export const ScriptPreview = () => {
	// needs backend support
	// would render http page for http endpoint, for example
	return "todo";
};

// TODO: use table instead of flex
export const RunLogs = (props: {
	pane: Extract<ScriptPaneT, { type: "run_logs" }>;
}) => {
	const s = useScript();
	const logs = useScriptLogs();
	const runsService = useScriptRuns();

	const { script_id, run_id } = props.pane;
	const channel_id = s.channel_id;

	const [logResource] = createResource(
		() => [channel_id, script_id, run_id] as const,
		([c, sid, rid]) => logs.list(c, sid, rid),
	);

	const [runInfo] = createResource(
		() => run_id,
		(rid) => runsService.fetch(`${channel_id}:${script_id}:${rid}`),
	);

	const [levelFilter, setLevelFilter] = createSignal<string>("all");
	const [expandedEntry, setExpandedEntry] = createSignal<number | null>(null);

	const filteredLogs = () => {
		const filter = levelFilter();
		if (filter === "all") return logs.getLogsForRun(run_id);
		return logs.getLogsForRun(run_id).filter((e) => e.level === filter);
	};

	const hasAttrs = (entry: { attributes?: Record<string, unknown> }) =>
		entry.attributes && Object.keys(entry.attributes).length > 0;

	const toggleExpand = (entryId: number) => {
		setExpandedEntry((prev) => (prev === entryId ? null : entryId));
	};

	const handleStop = async () => {
		await runsService.stop(channel_id, script_id, run_id);
	};

	const formatAttrsSummary = (attrs?: Record<string, unknown>) => {
		if (!attrs) return "";
		return Object.entries(attrs)
			.map(([key, val]) => {
				let valStr = String(val);
				if (valStr.length > 20) {
					valStr = valStr.substring(0, 17) + "...";
				}
				return `${key}=${valStr}`;
			})
			.join(" ");
	};

	return (
		<div class="run-logs">
			<Show when={logResource.loading}>
				<div>Loading logs...</div>
			</Show>
			<Show when={logResource.error}>
				<div>Error: {logResource.error}</div>
			</Show>
			<Show when={!logResource.loading && !logResource.error}>
				<Show when={runInfo()}>
					{(run) => (
						<div class="top">
							<span class="status" data-status={run().status}>
								{run().status}
							</span>
							<Show
								when={run().status === "Active" || run().status === "Creating"}
							>
								<button type="button" onClick={handleStop}>
									Stop
								</button>
							</Show>
						</div>
					)}
				</Show>
				<div class="log-filters">
					<button
						type="button"
						onClick={() => setLevelFilter("all")}
						aria-pressed={levelFilter() === "all"}
					>
						All
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Info")}
						aria-pressed={levelFilter() === "Info"}
					>
						Info
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Warning")}
						aria-pressed={levelFilter() === "Warning"}
					>
						Warning
					</button>
					<button
						type="button"
						onClick={() => setLevelFilter("Error")}
						aria-pressed={levelFilter() === "Error"}
					>
						Error
					</button>
				</div>
				<ul role="log">
					<For each={filteredLogs()}>
						{(entry) => (
							<li
								classList={{ expanded: expandedEntry() === entry.id }}
								onclick={() => toggleExpand(entry.id)}
								style="cursor: pointer"
							>
								<div class="main">
									<span class="time">
										<Time date={new Date(entry.created_at)} />
									</span>
									<span class="level" data-level={entry.level}>
										{entry.level}
									</span>
									<span class="content">{entry.content}</span>
									<Show when={hasAttrs(entry)}>
										<span class="attrs-summary">
											{formatAttrsSummary(entry.attributes)}
										</span>
									</Show>
								</div>
								<Show when={expandedEntry() === entry.id && hasAttrs(entry)}>
									<ul class="attrs expanded">
										<For each={Object.entries(entry.attributes ?? {})}>
											{([key, val]) => (
												<li>
													<span class="key">{key}</span>
													<span class="syn">=</span>
													<span class="val">{String(val)}</span>
												</li>
											)}
										</For>
									</ul>
								</Show>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
};
