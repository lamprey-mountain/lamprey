import { createSignal, createUniqueId, For } from "solid-js";
import { RadioDot } from "../icons";
import { Dropdown } from "../Dropdown";

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
			<div style="display:flex">
				<div style="flex: 1">Date format</div>
				<Dropdown
					options={[
						{ label: "dd/mm/yyyy", item: "standard" },
						{ label: "mm/dd/yyyy", item: "american" },
						{ label: "yyyy-mm-dd", item: "iso" },
					]}
				/>
			</div>
			<br />
			<div style="display:flex">
				<div style="flex: 1">Time format</div>
				<Dropdown
					options={[
						{ label: "auto", item: "auto" },
						{ label: "12h", item: "12h" },
						{ label: "24h", item: "24h" },
					]}
				/>
			</div>
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
