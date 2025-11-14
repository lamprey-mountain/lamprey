import { type VoidProps } from "solid-js";
import { type User } from "sdk";
import { Checkbox } from "../icons";
import { useCtx } from "../context.ts";
import { Dropdown } from "../Dropdown.tsx";

export function Chat(_props: VoidProps<{ user: User }>) {
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

	return (
		<div class="user-settings-chat">
			<h2>chat</h2>
			<br />
			<div class="options">
				<h3 class="dim" style="margin-top:0">media</h3>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["preview_attachments"] === "yes"}
						onInput={toggle("preview_attachments")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["preview_attachments"] === "yes"}
					/>
					<span>Preview attachments</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig()
							.frontend["preview_attachments_descriptions"] === "yes"}
						onInput={toggle("preview_attachments_descriptions")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig()
							.frontend["preview_attachments_descriptions"] === "yes"}
					/>
					<span>Show attachment descriptions (alt text)</span>
				</label>
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["link_previews"] === "yes"}
						onInput={toggle("link_previews")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["link_previews"] === "yes"}
					/>
					<span>Enable link previews</span>
				</label>
				<h3 class="dim">input</h3>
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
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["typing_indicators"] === "yes"}
						onInput={toggle("typing_indicators")}
						style="display: none;"
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["typing_indicators"] === "yes"}
					/>
					<span>Show typing indicators</span>
				</label>
				<h3 class="dim">content</h3>
				<div class="option apart">
					<div>
						<div>Show spoilers</div>
						<div class="dim">Show when to show spoilers</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().frontend["spoilers"] || "click"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.userConfig();
								ctx.setUserConfig({
									...c,
									frontend: {
										...c.frontend,
										spoilers: value,
									},
								});
							}
						}}
						options={[
							{ item: "click", label: "On click" },
							{ item: "hover", label: "On hover" },
							{ item: "always", label: "Always" },
						]}
					/>
				</div>
			</div>
		</div>
	);
}
