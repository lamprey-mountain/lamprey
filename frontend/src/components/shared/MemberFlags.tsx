import type { RoomMember, RoomMemberOrigin } from "sdk";
import { Show } from "solid-js";
import { formatDuration } from "@/atoms/DurationInput";
import { Icon } from "@/atoms/Icon";
import { getDate } from "@/utils/general";
import { icEye, icHeadphones, icMic, icWarning } from "@/utils/icons";

export function isTimedOut(member: RoomMember) {
	return (
		!!member.timeout_until &&
		getDate(member.timeout_until).valueOf() > Date.now()
	);
}

export function hasModerationFlags(member: RoomMember) {
	return member.mute || member.deaf || member.quarantined || isTimedOut(member);
}

export function timeoutDuration(member: RoomMember) {
	if (!member.timeout_until) return "timed out";
	const remaining = getDate(member.timeout_until).valueOf() - Date.now();
	return formatDuration(Math.max(0, Math.round(remaining / 1000)));
}

export function timeoutTooltip(member: RoomMember) {
	return `timed out for ${timeoutDuration(member)}`;
}

export function formatOrigin(o: RoomMemberOrigin | undefined | null) {
	switch (o?.type) {
		case "Invite":
			return o.code;
		case "BotInstall":
			return "bot install";
		case "Bridged":
			return "bridged";
		case "Creator":
			return "room creator";
		case "GdmUpgrade":
			return "gdm upgrade";
		case "PublicJoin":
			return "public join";
		case null:
		case undefined:
			return "unknown";
	}
}

// moderation status icons for a room member
export function MemberFlags(props: { member: RoomMember }) {
	return (
		<div class="member-flags">
			<Show when={props.member.mute}>
				<div class="member-flag muted" data-tooltip="room muted">
					<Icon src={icMic} alt="muted" />
				</div>
			</Show>
			<Show when={props.member.deaf}>
				<div class="member-flag deafened" data-tooltip="room deafened">
					<Icon src={icHeadphones} alt="deafened" />
				</div>
			</Show>
			<Show when={isTimedOut(props.member)}>
				<div
					class="member-flag timed-out"
					data-tooltip={timeoutTooltip(props.member)}
				>
					<Icon src={icWarning} alt="timed out" />
				</div>
			</Show>
			<Show when={props.member.quarantined}>
				<div class="member-flag quarantined" data-tooltip="quarantined">
					<Icon src={icEye} alt="quarantined" />
				</div>
			</Show>
		</div>
	);
}
