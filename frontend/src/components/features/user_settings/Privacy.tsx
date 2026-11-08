import type { User } from "sdk";
import { onMount, Show, type VoidProps } from "solid-js";
import { type Harvest, useApi } from "@/api";
import { useCtx } from "@/app/context";
import { ApiErrorDisplay, isApiError } from "@/atoms/ApiErrorDisplay";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Time } from "@/atoms/Time";
import { useModals } from "@/contexts/modal";
import { formatBytes } from "@/media/util.tsx";
import { getDate } from "@/utils/general";

export function Privacy(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();
	const { t } = useCtx();
	const [, modalCtl] = useModals();

	const harvest = () => api.harvest.current();
	const exportInProgress = () => {
		const h = harvest();
		return h?.status === "Queued" || h?.status === "Archiving";
	};
	const harvestError = () => {
		const err = api.harvest.error();
		return isApiError(err) ? err : null;
	};

	onMount(() => {
		void api.harvest.load();
	});

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

				<br />
				<h3>{t("user_settings.data_export")}</h3>
				<p>{t("user_settings.data_export_description")}</p>
				<button
					type="button"
					class="button"
					disabled={exportInProgress()}
					onClick={() => modalCtl.open({ type: "export_data" })}
				>
					{t("user_settings.data_export")}
				</button>
				<Show when={api.harvest.loading()}>
					<p class="dim">loading export status...</p>
				</Show>
				<Show when={harvestError()}>
					{(err) => <ApiErrorDisplay error={err()} />}
				</Show>
				<Show when={api.harvest.error() && !harvestError()}>
					<p class="error">failed to load export status</p>
				</Show>
				<Show when={harvest()}>{(h) => <HarvestInfo harvest={h()} />}</Show>
			</div>
		</div>
	);
}

function HarvestInfo(props: { harvest: Harvest }) {
	const h = props.harvest;

	switch (h.status) {
		case "Queued":
			return (
				<p>
					Your export is queued (<Time date={getDate(h.queued_at)} />
					).
				</p>
			);
		case "Archiving":
			return (
				<p>
					Your export is being generated. Started{" "}
					<Time date={getDate(h.started_at)} />.
					{h.eta_at && (
						<>
							{" "}
							Estimated to finish <Time date={getDate(h.eta_at)} />.
						</>
					)}
				</p>
			);
		case "Completed":
			return (
				<div class="harvest-info">
					<p>
						Your export is ready ({formatBytes(h.size)}). It expires{" "}
						<Time date={getDate(h.expires_at)} />.
					</p>
					<a
						class="button primary"
						href={h.url}
						target="_blank"
						rel="noreferrer"
					>
						download
					</a>
				</div>
			);
		case "Failed":
			return (
				<p class="error">
					Your export failed (<Time date={getDate(h.failed_at)} />
					): {h.message}
				</p>
			);
		case "Cancelled":
			return (
				<p>
					Your export was cancelled (<Time date={getDate(h.cancelled_at)} />
					): {h.message}
				</p>
			);
	}
}
