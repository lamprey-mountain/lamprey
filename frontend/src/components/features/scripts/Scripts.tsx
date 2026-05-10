import { Show } from "solid-js";
import type { Channel } from "ts-sdk";
import { useScriptLogs } from "@/api";
import { Time } from "@/atoms/Time";
import { createScriptContext, ScriptContext, useScript } from "./context";

// in channel nav: show current script like a thread

export const Scripts = (props: { channel: Channel }) => {
	const s = createScriptContext();
	const logs = useScriptLogs();

	const openScript = (scriptId: string) => {
		s.reset();
		s.createPane({
			id: 0,
			type: "split_horizontal",
		});
		s.createPane({
			id: 1,
			parentId: 0,
			type: "script_code",
		});
		s.createPane({
			id: 2,
			parentId: 0,
			type: "script_inputs",
		});
		logs.subscribe(props.channel.id, scriptId);
	};

	return (
		<ScriptContext.Provider value={s}>
			<div style="grid-area:main">
				<Show when={false /* no open panes */}>
					<div>search bar</div>
					<ul>
						<li>
							<div>script 1</div>
							<div>script 2</div>
							<div>script 3</div>
						</li>
					</ul>
				</Show>
				{/* otherwise render panes here*/}
			</div>
		</ScriptContext.Provider>
	);
};

export const ScriptPane = () => {
	return (
		<div>
			<header>
				<nav>dropdown, switch pane type</nav>
				<div>pane title</div>
			</header>
			<div>render actual contents here</div>
		</div>
	);
};

export const ScriptCode = () => {
	// todo install codemirror

	return (
		<div>
			{/* anything else here? like save button etc? */}
			<div>codemirror editor here</div>
		</div>
	);
};

export const ScriptInputs = () => {
	// ul containing li of inputs, input type
	// if input type = trigger show a button
	// show ul showing recent runs
	return "todo: render";
};

export const ScriptPreview = () => {
	// needs backend support
	// would render http page for http endpoint, for example
	return "todo";
};

export const RunLogs = () => {
	return (
		<div>
			<div>todo: metrics here</div>
			<ul role="log">
				<li style="background:red;display:grid;grid-template-columns:auto 1fr;grid-template-rows:1fr auto;grid-template-areas: 'date content' '. attrs';">
					<div style="grid-area:date">
						<Time date={new Date()} />
					</div>
					<div style="grid-area:content;display:flex">
						<div style="color:blue">info</div>
						text here
						<Show when={false /* not expanded*/}>
							<span class="attrs">
								<span>
									<span class="key">key</span>
									<span class="syn">=</span>
									<span class="val">val</span>
								</span>
								<span>
									<span class="key">key</span>
									<span class="syn">=</span>
									<span class="val">val</span>
								</span>
							</span>
						</Show>
					</div>
					<Show when={false /* expanded*/}>
						<div style="grid-area:attrs;">
							<ul>
								<li>
									<span class="key">key</span>
									<span class="syn">=</span>
									<span class="val">val</span>
								</li>
								<li>
									<span class="key">foo</span>
									<span class="syn">=</span>
									<span class="val">bar</span>
								</li>
							</ul>
						</div>
					</Show>
				</li>
			</ul>
		</div>
	);
};
