import fuzzysort from "fuzzysort";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import type { Channel, Script } from "ts-sdk";
import { useScriptLogs, useScriptRuns, useScripts } from "@/api";
import { Time } from "@/atoms/Time";
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
	};

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
	const size = () => props.pane.size;

	return (
		<div
			class="pane-container"
			style={{
				flex: size() ? "none" : "1",
				width: size() && props.isHorizontal ? `${size}px` : undefined,
				height:
					size() && props.isHorizontal === false ? `${size}px` : undefined,
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
						{(child) => (
							<ScriptPaneRenderer
								pane={child}
								isHorizontal={props.pane.type === "split_horizontal"}
							/>
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

	// TODO: use x icons for pane close button

	return (
		<div class="script-pane">
			<header>
				<nav>{pane.type.replace("script_", "").replace("_", " ")}</nav>
				<div class="title">Pane {pane.id}</div>
				<button
					type="button"
					class="close"
					onClick={() => s.closePane(pane.id)}
				>
					&times;
				</button>
			</header>
			<div class="pane-content">
				<Switch>
					<Match when={pane.type === "script_code"}>
						<ScriptCode />
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

export const ScriptCode = () => {
	// anything else here? like save button etc?

	return (
		<div>
			<LazyCodeEditor />
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
					<For each={script()?.inputs}>
						{(input) => (
							<div class="script-input" data-input-type={input.type.type}>
								<Show when={input.type?.type === "Manual"}>
									<button
										class="inner"
										type="button"
										onClick={() => trigger(input.id)}
									>
										<div>{input.label}</div>
										<div class="dim">{input.id}</div>
									</button>
								</Show>
								<Show when={input.type?.type !== "Manual"}>
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

	const filteredLogs = () => {
		const filter = levelFilter();
		if (filter === "all") return logs.getLogsForRun(run_id);
		return logs.getLogsForRun(run_id).filter((e) => e.level === filter);
	};

	const handleStop = async () => {
		await runsService.stop(channel_id, script_id, run_id);
	};

	return (
		<div>
			<Show when={logResource.loading}>
				<div>Loading logs...</div>
			</Show>
			<Show when={logResource.error}>
				<div>Error: {logResource.error}</div>
			</Show>
			<Show when={!logResource.loading && !logResource.error}>
				<div>
					<Show when={runInfo()}>
						{(run) => (
							<div>
								<span class="status" data-status={run().status}>
									{run().status}
								</span>
								<Show
									when={
										run().status === "Active" || run().status === "Creating"
									}
								>
									<button type="button" onClick={handleStop}>
										Stop
									</button>
								</Show>
							</div>
						)}
					</Show>
					<div>
						<button type="button" onClick={() => setLevelFilter("all")}>
							All
						</button>
						<button type="button" onClick={() => setLevelFilter("Info")}>
							Info
						</button>
						<button type="button" onClick={() => setLevelFilter("Warn")}>
							Warn
						</button>
						<button type="button" onClick={() => setLevelFilter("Error")}>
							Error
						</button>
					</div>
					<ul role="log">
						<For each={filteredLogs()}>
							{(entry) => (
								<li>
									<Time date={new Date(entry.time)} />
									<span class="level" data-level={entry.level}>
										{entry.level}
									</span>
									<span>{entry.content}</span>
									<Show
										when={entry.attrs && Object.keys(entry.attrs).length > 0}
									>
										<span class="attrs">
											<For each={Object.entries(entry.attrs)}>
												{([key, val]) => (
													<span>
														<span class="key">{key}</span>
														<span>=</span>
														<span class="val">{String(val)}</span>
													</span>
												)}
											</For>
										</span>
									</Show>
								</li>
							)}
						</For>
					</ul>
				</div>
			</Show>
		</div>
	);
};
