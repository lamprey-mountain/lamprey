import { Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useCtx } from "../../../context.ts";
import { useApi2 } from "@/api";
import { getThumbFromId } from "../../../media/util.tsx";
import { Checkbox } from "../../../icons";
import { Dropdown } from "../../../atoms/Dropdown";
import { CheckboxOption } from "../../../atoms/CheckboxOption";

export function Appearance(props: VoidProps<{ user: User }>) {
	const api2 = useApi2();
	const ctx = useCtx();
	const { t } = useCtx();

	const toggle = (setting: string) => () => {
		const c = ctx.preferences();
		ctx.setPreferences({
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
					selected={ctx.preferences().frontend["theme"] || "auto"}
					onSelect={(value) => {
						if (value) {
							const c = ctx.preferences();
							ctx.setPreferences({
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
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-underline-links`}
					checked={ctx.preferences().frontend["underline_links"] === "yes"}
					onChange={() => toggle("underline_links")()}
					seed={`user-${props.user?.id ?? "@self"}-underline-links`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["underline_links"] === "yes"}
						seed={`user-${props.user?.id ?? "@self"}-underline-links`}
					/>
					<span>{t("user_settings.underline_links")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-show-send-button`}
					checked={ctx.preferences().frontend["show_send_button"] === "yes"}
					onChange={() => toggle("show_send_button")()}
					seed={`user-${props.user?.id ?? "@self"}-show-send-button`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["show_send_button"] === "yes"}
						seed={`user-${props.user?.id ?? "@self"}-show-send-button`}
					/>
					<span>{t("user_settings.show_send_button")}</span>
				</CheckboxOption>
				<div class="option apart">
					<div>
						<div>{t("user_settings.message_style")}</div>
						<div class="dim">
							{t("user_settings.message_style_description") ||
								"Choose how messages are displayed"}
						</div>
					</div>
					<Dropdown
						selected={ctx.preferences().frontend["message_style"] || "cozy"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.preferences();
								ctx.setPreferences({
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
						value={(ctx.preferences().frontend["message_spacing"] as
							| number
							| undefined) || 8}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
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
						value={(ctx.preferences().frontend["chat_font_scale"] as
							| number
							| undefined) || 100}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
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
						value={(ctx.preferences().frontend["app_scale"] as
							| number
							| undefined) || 100}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
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
						value={(ctx.preferences().frontend["saturation"] as
							| number
							| undefined) || 100}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
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
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-reduced-motion`}
					checked={ctx.preferences().frontend["reduced_motion"] === "yes"}
					onChange={() => toggle("reduced_motion")()}
					seed={`user-${props.user?.id ?? "@self"}-reduced-motion`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["reduced_motion"] === "yes"}
						seed={`user-${props.user?.id ?? "@self"}-reduced-motion`}
					/>
					<span>{t("user_settings.reduced_motion")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-reduced-motion-sync`}
					checked={ctx.preferences().frontend["reduced_motion_sync"] === "yes"}
					onChange={() => toggle("reduced_motion_sync")()}
					seed={`user-${props.user?.id ?? "@self"}-reduced-motion-sync`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["reduced_motion_sync"] ===
							"yes"}
						seed={`user-${props.user?.id ?? "@self"}-reduced-motion-sync`}
					/>
					<span>{t("user_settings.reduced_motion_sync")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-autoplay-gifs`}
					checked={ctx.preferences().frontend["autoplay_gifs"] === "yes"}
					onChange={() => toggle("autoplay_gifs")()}
					seed={`user-${props.user?.id ?? "@self"}-autoplay-gifs`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["autoplay_gifs"] === "yes"}
						seed={`user-${props.user?.id ?? "@self"}-autoplay-gifs`}
					/>
					<span>{t("user_settings.autoplay_gifs")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-autoplay-emoji`}
					checked={ctx.preferences().frontend["autoplay_emoji"] === "yes"}
					onChange={() => toggle("autoplay_emoji")()}
					seed={`user-${props.user?.id ?? "@self"}-autoplay-emoji`}
				>
					<Checkbox
						checked={ctx.preferences().frontend["autoplay_emoji"] === "yes"}
						seed={`user-${props.user?.id ?? "@self"}-autoplay-emoji`}
					/>
					<span>{t("user_settings.autoplay_emoji")}</span>
				</CheckboxOption>
			</div>
		</div>
	);
}
