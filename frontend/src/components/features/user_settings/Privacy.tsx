import type { User } from "sdk";
import type { VoidProps } from "solid-js";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";

export function Privacy(props: VoidProps<{ user: User }>) {
	const ctx = useCtx();
	const { t } = useCtx();

	const togglePrivacy = (key: "dms" | "rpc" | "exif") => () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			privacy: {
				...c.privacy,
				[key]: !c.privacy[key],
			},
		});
	};

	const toggleFriends =
		(key: "allow_everyone" | "allow_mutual_room" | "allow_mutual_friend") =>
		() => {
			const c = ctx.preferences();
			ctx.setPreferences({
				...c,
				privacy: {
					...c.privacy,
					friends: {
						...c.privacy.friends,
						[key]: !c.privacy.friends[key],
					},
				},
			});
		};

	// TODO: use <label>s for everything
	return (
		<div class="user-settings-privacy">
			<h2>{t("user_settings.privacy")}</h2>
			<br />
			<div class="options">
				<h3>{t("user_settings.privacy_friends")}</h3>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-allow-everyone`}
					checked={ctx.preferences().privacy.friends.allow_everyone}
					onChange={toggleFriends("allow_everyone")}
					seed={`user-${props.user?.id ?? "@self"}-allow-everyone`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.friends.allow_everyone}
						seed={`user-${props.user?.id ?? "@self"}-allow-everyone`}
					/>
					<span>{t("user_settings.privacy_allow_everyone")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-allow-mutual-room`}
					checked={ctx.preferences().privacy.friends.allow_mutual_room}
					onChange={toggleFriends("allow_mutual_room")}
					seed={`user-${props.user?.id ?? "@self"}-allow-mutual-room`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.friends.allow_mutual_room}
						seed={`user-${props.user?.id ?? "@self"}-allow-mutual-room`}
					/>
					<span>{t("user_settings.privacy_allow_mutual_room")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`user-${props.user?.id ?? "@self"}-allow-mutual-friend`}
					checked={ctx.preferences().privacy.friends.allow_mutual_friend}
					onChange={toggleFriends("allow_mutual_friend")}
					seed={`user-${props.user?.id ?? "@self"}-allow-mutual-friend`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.friends.allow_mutual_friend}
						seed={`user-${props.user?.id ?? "@self"}-allow-mutual-friend`}
					/>
					<span>{t("user_settings.privacy_allow_mutual_friend")}</span>
				</CheckboxOption>

				<br />
				<h3>{t("user_settings.privacy_rooms")}</h3>
				<p>{t("user_settings.privacy_rooms_description")}</p>
				<CheckboxOption
					id={`preferences-dms`}
					checked={ctx.preferences().privacy.dms}
					onChange={togglePrivacy("dms")}
					seed={`preferences-dms`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.dms}
						seed={`preferences-dms`}
					/>
					<span>{t("user_settings.privacy_dms")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`preferences-rpc`}
					checked={ctx.preferences().privacy.rpc}
					onChange={togglePrivacy("rpc")}
					seed={`preferences-rpc`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.rpc}
						seed={`preferences-rpc`}
					/>
					<span>{t("user_settings.privacy_rpc")}</span>
				</CheckboxOption>
				<CheckboxOption
					id={`preferences-exif`}
					checked={ctx.preferences().privacy.exif}
					onChange={togglePrivacy("exif")}
					seed={`preferences-exif`}
				>
					<Checkbox
						checked={ctx.preferences().privacy.exif}
						seed={`preferences-exif`}
					/>
					<label for={`preferences-exif`}>
						<div>{t("user_settings.privacy_exif")}</div>
						<div class="dim">{t("user_settings.privacy_exif_description")}</div>
					</label>
				</CheckboxOption>
			</div>
		</div>
	);
}
