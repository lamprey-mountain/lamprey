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
	const { t } = useCtx();

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
	// TODO: message group spacing
	// TODO: chat font scale
	// TODO: application scale
	// TODO: saturation
	// TODO: reduced motion (sync with computer, autoplay gifs, emoji)

	return (
		<div class="user-settings-appearance">
			<h2>{t("user_settings.appearance")}</h2>
			<br />
			<div class="option apart">
				<div>
					<div>{t("user_settings.theme")}</div>
					<div class="dim">
						{t("user_settings.theme_description") ||
							"Choose your preferred theme"}
					</div>
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
						{
							item: "auto",
							label: t("user_settings.theme_auto") || "Auto (system)",
						},
						{
							item: "auto-highcontrast",
							label: t("user_settings.theme_auto_highcontrast") ||
								"Auto (system) (high contrast)",
						},
						{ item: "light", label: t("user_settings.theme_light") || "Light" },
						{ item: "dark", label: t("user_settings.theme_dark") || "Dark" },
						{
							item: "light-highcontrast",
							label: t("user_settings.theme_light_highcontrast") ||
								"Light (high contrast)",
						},
						{
							item: "dark-highcontrast",
							label: t("user_settings.theme_dark_highcontrast") ||
								"Dark (high contrast)",
						},
					]}
				/>
			</div>
			<div class="options">
				<label class="option">
					<input
						type="checkbox"
						checked={ctx.userConfig().frontend["underline_links"] === "yes"}
						onInput={toggle("underline_links")}
					/>
					<Checkbox
						checked={ctx.userConfig().frontend["underline_links"] === "yes"}
					/>
					<span>{t("user_settings.underline_links")}</span>
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
					<span>{t("user_settings.show_send_button")}</span>
				</label>
				<div class="option apart">
					<div>
						<div>{t("user_settings.message_style")}</div>
						<div class="dim">
							{t("user_settings.message_style_description") ||
								"Choose how messages are displayed"}
						</div>
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
							{
								item: "cozy",
								label: t("user_settings.message_style_cozy") || "Cozy",
							},
							{
								item: "compact",
								label: t("user_settings.message_style_compact") || "Compact",
							},
						]}
					/>
				</div>
				<div class="option apart">
					<div>
						<div>{t("user_settings.message_group_spacing")}</div>
						<div class="dim">
							{t("user_settings.message_group_spacing_description") ||
								"Adjust the spacing between message groups"}
						</div>
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
						<div>{t("user_settings.chat_font_scale")}</div>
						<div class="dim">
							{t("user_settings.chat_font_scale_description") ||
								"Adjust the size of chat text"}
						</div>
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
						<div>{t("user_settings.application_scale")}</div>
						<div class="dim">
							{t("user_settings.application_scale_description") ||
								"Adjust the overall application size"}
						</div>
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
						<div>{t("user_settings.saturation")}</div>
						<div class="dim">
							{t("user_settings.saturation_description") ||
								"Adjust the color saturation"}
						</div>
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
				<h3 class="dim">{t("user_settings.reduced_motion")}</h3>
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
					<span>{t("user_settings.reduced_motion")}</span>
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
					<span>{t("user_settings.reduced_motion_sync")}</span>
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
					<span>{t("user_settings.autoplay_gifs")}</span>
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
					<span>{t("user_settings.autoplay_emoji")}</span>
				</label>
			</div>
		</div>
	);
}
