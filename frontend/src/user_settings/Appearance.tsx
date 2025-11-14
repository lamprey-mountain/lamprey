import { Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { getThumbFromId } from "../media/util.tsx";
import { Checkbox } from "../icons";
import { Dropdown } from "../Dropdown";

export function Appearance(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const toggle = (setting: string) => () => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				[setting]: c.frontend[setting] === "yes" ? "no" : "yes",
			},
		});
	};

	// TODO(#429): auto, light, dark mode themes
	// TODO(#429): theme accent color
	// TODO(#429): high contrast mode
	// TODO: preview message styling
	// TODO: show send message button
	// TODO: compact/cozy message style
	// TODO: message group spacing
	// TODO: chat font scale
	// TODO: application scale
	// TODO: saturation
	// TODO: reduced motion (sync with computer, autoplay gifs, emoji)

	return (
		<div class="user-settings-appearance">
			<h2>appearance</h2>
			<br />
			<div class="option apart">
				<div>
					<div>Theme</div>
					<div class="dim">Choose your preferred theme</div>
				</div>
				<Dropdown
					selected={ctx.userConfig().frontend["theme"] || "auto"}
					onSelect={(value) => {
						if (value) {
							const c = ctx.userConfig();
							ctx.setUserConfig({
								...c,
								frontend: {
									...c.frontend,
									theme: value,
								},
							});
						}
					}}
					options={[
						{ item: "auto", label: "Auto (system)" },
						{
							item: "auto-highcontrast",
							label: "Auto (system) (high contrast)",
						},
						{ item: "light", label: "Light" },
						{ item: "dark", label: "Dark" },
						{ item: "light-highcontrast", label: "Light (high contrast)" },
						{ item: "dark-highcontrast", label: "Dark (high contrast)" },
					]}
				/>
			</div>
			<div class="options">
				{/* TODO: merge this into "compact/cozy" mode */}
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["message_pfps"] === "yes"}
						onInput={toggle("message_pfps")}
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["message_pfps"] === "yes"}
					/>
					<span>Show pfps in messages (experimental)</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["underline_links"] === "yes"}
						onInput={toggle("underline_links")}
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["underline_links"] === "yes"}
					/>
					<span>Always underline links</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["show_send_button"] === "yes"}
						onInput={toggle("show_send_button")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["show_send_button"] === "yes"}
					/>
					<span>Show send message button</span>
				</label>
				<div class="option apart">
					<div>
						<div>Message style</div>
						<div class="dim">Choose how messages are displayed</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().frontend["message_style"] || "cozy"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.userConfig();
								ctx.setUserConfig({
									...c,
									frontend: {
										...c.frontend,
										message_style: value,
									},
								});
							}
						}}
						options={[
							{ item: "cozy", label: "Cozy" },
							{ item: "compact", label: "Compact" },
						]}
					/>
				</div>
				<div class="option apart">
					<div>
						<div>Message group spacing</div>
						<div class="dim">Adjust the spacing between message groups</div>
					</div>
					<input
						type="range"
						min="0"
						max="24"
						value={ctx.userConfig().frontend["message_spacing"] || 8}
						onChange={(e) => {
							const c = ctx.userConfig();
							ctx.setUserConfig({
								...c,
								frontend: {
									...c.frontend,
									["message_spacing"]: e.target.value,
								},
							});
						}}
						class="slider"
					/>
				</div>
				<div class="option apart">
					<div>
						<div>Chat font scale</div>
						<div class="dim">Adjust the size of chat text</div>
					</div>
					<input
						type="range"
						min="80"
						max="150"
						value={ctx.userConfig().frontend["chat_font_scale"] || 100}
						onChange={(e) => {
							const c = ctx.userConfig();
							ctx.setUserConfig({
								...c,
								frontend: {
									...c.frontend,
									["chat_font_scale"]: e.target.value,
								},
							});
						}}
						class="slider"
					/>
				</div>
				<div class="option apart">
					<div>
						<div>Application scale</div>
						<div class="dim">Adjust the overall application size</div>
					</div>
					<input
						type="range"
						min="80"
						max="150"
						value={ctx.userConfig().frontend["app_scale"] || 100}
						onChange={(e) => {
							const c = ctx.userConfig();
							ctx.setUserConfig({
								...c,
								frontend: {
									...c.frontend,
									["app_scale"]: e.target.value,
								},
							});
						}}
						class="slider"
					/>
				</div>
				<div class="option apart">
					<div>
						<div>Saturation</div>
						<div class="dim">Adjust the color saturation</div>
					</div>
					<input
						type="range"
						min="0"
						max="100"
						value={ctx.userConfig().frontend["saturation"] || 100}
						onChange={(e) => {
							const c = ctx.userConfig();
							ctx.setUserConfig({
								...c,
								frontend: {
									...c.frontend,
									["saturation"]: e.target.value,
								},
							});
						}}
						class="slider"
					/>
				</div>
				<h3 class="dim">reduced motion</h3>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["reduced_motion"] === "yes"}
						onInput={toggle("reduced_motion")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["reduced_motion"] === "yes"}
					/>
					<span>Reduced motion</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["reduced_motion_sync"] ===
							"yes"}
						onInput={toggle("reduced_motion_sync")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["reduced_motion_sync"] ===
							"yes"}
					/>
					<span>Sync reduced motion with system settings</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["autoplay_gifs"] === "yes"}
						onInput={toggle("autoplay_gifs")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["autoplay_gifs"] === "yes"}
					/>
					<span>Autoplay GIFs in messages</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["autoplay_emoji"] === "yes"}
						onInput={toggle("autoplay_emoji")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["autoplay_emoji"] === "yes"}
					/>
					<span>Autoplay animated emoji</span>
				</label>
			</div>
		</div>
	);
}
