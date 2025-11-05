import { createSignal, createUniqueId, For, Show } from "solid-js";

const langs = [
	{ labelNative: "english", labelLocalized: "english", id: "en-US" },
	{ labelNative: "englash", labelLocalized: "also english", id: "en-US2" },
	{ labelNative: "englosh", labelLocalized: "still english", id: "en-US3" },
];

export function Language() {
	const [selectedLang, setSelectedLang] = createSignal("en-US");
	const radioId = createUniqueId();

	return (
		<div class="user-settings-lang">
			<h2>language</h2>
			<br />
			<div>select date format (dd/mm/yyyy, mm/dd/yyyy, yyyy-mm-dd)</div>
			<div>select time format (auto, 12h, 24h)</div>
			<br />
			<ul class="langs">
				<For each={langs}>
					{(lang) => (
						<label class="lang">
							<input
								type="radio"
								name={radioId}
								onInput={() => setSelectedLang(lang.id)}
							/>
							<RadioDot checked={selectedLang() === lang.id} />
							<div>
								<div>{lang.labelLocalized}</div>
								<div class="dim">{lang.labelNative}</div>
							</div>
						</label>
					)}
				</For>
			</ul>
		</div>
	);
}

const RadioDot = (props: { checked?: boolean }) => {
	return (
		<svg
			class="radio"
			viewBox="0 0 16 16"
			aria-hidden="true"
			xmlns="http://www.w3.org/2000/svg"
		>
			<circle
				cx="8"
				cy="8"
				r="6"
				fill={props.checked ? "oklch(var(--color-link-200))" : "none"}
				stroke="currentColor"
				stroke-width="1"
			/>
			<Show when={props.checked}>
				<circle cx="8" cy="8" r="3" fill="currentColor" />
			</Show>
		</svg>
	);
};
