import { debounce } from "@solid-primitives/scheduled";
import { useNavigate } from "@solidjs/router";
import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import type { Invite, InviteTarget } from "ts-sdk";
import { useApi } from "@/api";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown";
import { useModals } from "@/contexts/modal";
import { createResizeTransition } from "@/hooks/createResizeTransition";
import { getThumbFromId } from "@/media/util";
import { icChevron, icWarning } from "@/utils/icons";
import { Modal } from "./mod";

type ModalRoomCreateOrJoinProps = {};

type InviteError = { code: string; message: string };
type InviteResult = Invite | { error: InviteError } | undefined | null;

const parseInvite = (s: string) => {
	return s.match(/\/invite\/([a-z0-9]+)/i)?.[1] ?? s;
};

const matchInviteSuccess = (i: InviteResult): Invite | false => {
	if (!i) return false;
	return "code" in i ? i : false;
};

const matchInviteError = (i: InviteResult): InviteError | false => {
	if (!i) return false;
	return "error" in i ? i.error : false;
};

export const ModalRoomCreateOrJoin = (_props: ModalRoomCreateOrJoinProps) => {
	const api = useApi();
	const [, modalCtl] = useModals();
	const nav = useNavigate();

	const [view, setView] = createSignal<"selection" | "create" | "invite">(
		"selection",
	);
	const [inviteCode, setInviteCode] = createSignal("");
	const [roomName, setRoomName] = createSignal("");
	const [isPublic, setIsPublic] = createSignal(false);
	const [loading, setLoading] = createSignal(false);

	const [debouncedInviteCode, setDebouncedInviteCode] = createSignal("");
	const debouncedSetInviteCode = debounce(
		(value: string) => setDebouncedInviteCode(value),
		300,
	);

	const resizeTn = createResizeTransition();

	const [invite] = createResource<InviteResult, string>(
		debouncedInviteCode,
		async (code) => {
			const parsed = parseInvite(code);
			if (!parsed) return null;
			return api.invites.fetch(parsed).catch((err) => ({ error: err }));
		},
	);

	const handleCreate = async () => {
		if (loading()) return;
		const name = roomName().trim();
		if (!name) return; // TODO: "invalid input" ui
		setLoading(true);
		const room = await api.rooms.create({ name, public: isPublic() });
		modalCtl.close();
		nav(`/room/${room.id}`);
	};

	const handleInvite = async () => {
		if (loading()) return;
		const code = parseInvite(inviteCode());
		const resolved = invite();
		if (!code) return; // TODO: "invalid input" ui
		if (!resolved) return; // TODO: "invalid input" ui
		if ("error" in resolved) return; // TODO: "invalid input" ui
		setLoading(true);
		await api.invites.accept(code);
		modalCtl.close();

		// FIXME: update RoomNav selected room

		const t = resolved.target;
		switch (t.type) {
			case "User":
				return nav(`/user/${t.user.id}`);
			case "Room":
				return nav(
					t.channel?.id ? `/channel/${t.channel.id}` : `/room/${t.room.id}`,
				);
			case "Gdm":
				return nav(`/channel/${t.channel.id}`);
			case "Server":
				return nav("/");
		}
	};

	const renderError = (code: string) => {
		switch (code) {
			case "UnknownInvite":
				return "Invite code not found";
			default:
				return code;
		}
	};

	return (
		<Modal
			class="room-create-or-join unpadded"
			contentRef={(el) => {
				resizeTn.container(el);
				resizeTn.content(el.querySelector(".inner")!);
			}}
		>
			<Switch>
				<Match when={view() === "selection"}>
					<div class="main">
						<div class="selection">
							<button
								type="button"
								class="button"
								onClick={() => setView("create")}
							>
								{/* <Icon src={icAdd} /> */}
								create a room
								<div style="flex:1"></div>
								<Icon class="chevron" src={icChevron} />
							</button>
							<button
								type="button"
								class="button"
								onClick={() => setView("invite")}
							>
								use invite
								<div style="flex:1"></div>
								<Icon class="chevron" src={icChevron} />
							</button>
						</div>
					</div>
					<div class="bottom">
						<button type="button" class="link" onClick={() => modalCtl.close()}>
							cancel
						</button>
					</div>
				</Match>

				<Match when={view() === "create"}>
					<div class="main">
						<h3 classList={{ unnamed: !roomName().trim() }}>
							{roomName().trim() || "new room"}
						</h3>

						<form
							class="room-form"
							onSubmit={(e) => {
								e.preventDefault();
								handleCreate();
							}}
						>
							<label>
								<h3 class="dim">room name</h3>
								<input
									class="room-name-input"
									type="text"
									value={roomName()}
									onInput={(e) => setRoomName(e.currentTarget.value)}
									placeholder="my awesome room"
									required
									disabled={loading()}
									ref={(el) => queueMicrotask(() => el.focus())}
								/>
							</label>

							<CheckboxOptionWithLabel
								id="room-public"
								disabled={loading()}
								checked={isPublic()}
								onChange={setIsPublic}
								seed="public"
								label="public room"
								description="this room will be visible to and joinable by everyone"
							/>
						</form>
					</div>

					<div class="bottom">
						<button
							type="button"
							class="link"
							disabled={loading()}
							onClick={[setView, "selection"]}
						>
							Cancel
						</button>
						<button
							type="button"
							class="button primary"
							disabled={loading()}
							onClick={handleCreate}
						>
							{loading() ? "Creating Room..." : "Create Room"}
						</button>
					</div>
				</Match>

				<Match when={view() === "invite"}>
					<div class="main">
						<h3>use invite</h3>
						<form
							class="invite-form"
							onSubmit={(e) => {
								e.preventDefault();
								handleInvite();
							}}
						>
							<label>
								<h3 class="dim">invite code</h3>
								<input
									class="invite-input"
									type="text"
									value={inviteCode()}
									onInput={(e) => {
										setInviteCode(e.currentTarget.value);
										debouncedSetInviteCode(e.currentTarget.value);
									}}
									placeholder="a1b2c3"
									required
									disabled={loading()}
									ref={(el) => queueMicrotask(() => el.focus())}
								/>
							</label>
						</form>

						<Show when={invite() || invite.loading}>
							<div class="invite-target">
								<h3 class="dim header">invite target</h3>

								<div
									class="invite"
									classList={{
										loading: invite.loading,
										error: !!matchInviteError(invite()),
									}}
								>
									<Show when={matchInviteError(invite())}>
										{(err) => (
											<>
												<div class="error-message">
													<Icon src={icWarning} />
													<div>
														<div>
															<span class="error-prefix">Error:</span>{" "}
															{renderError(err().code)}
														</div>
														<div class="error-code">{err().code}</div>
													</div>
												</div>
											</>
										)}
									</Show>

									<Show when={matchInviteSuccess(invite())}>
										{(invite) => {
											function isRoomTarget(
												target: InviteTarget,
											): target is Extract<InviteTarget, { type: "Room" }> {
												return target.type === "Room";
											}

											function getRoomFromTarget(
												target: InviteTarget | undefined,
											) {
												if (target && isRoomTarget(target)) return target.room;
												return undefined;
											}

											const target = () => invite()?.target;
											const room = () =>
												target() && getRoomFromTarget(target());
											const roomIcon = () => room()?.icon;
											const roomDescription = () => room()?.description ?? "";
											const roomMemberCount = () => room()?.member_count ?? 0;
											const roomOnlineCount = () => room()?.online_count ?? 0;

											const name = () => {
												const i = invite();
												if (!i) return "unknown";
												const target = i.target;
												switch (target.type) {
													case "Room":
														return target.room?.name;
													case "Gdm":
														return target.channel?.name;
													case "Server":
														return "a server";
													case "User":
														return target.user?.name;
													default:
														return "unknown";
												}
											};

											return (
												<>
													<Show when={roomIcon()}>
														{(icon) => (
															<img
																src={getThumbFromId(icon(), 64)}
																class="avatar"
																alt={`${name()} room icon`}
															/>
														)}
													</Show>
													<div class="info">
														<div class="name">{name()}</div>
														<Show when={target()?.type === "Room"}>
															<Markdown
																content={roomDescription()}
																class="markdown"
															/>
															<div class="dim">
																{roomMemberCount()} members, {roomOnlineCount()}{" "}
																online
															</div>
														</Show>
													</div>
												</>
											);
										}}
									</Show>
								</div>
							</div>
						</Show>
					</div>

					<div class="bottom">
						<button
							type="button"
							class="link"
							disabled={loading()}
							onClick={[setView, "selection"]}
						>
							Cancel
						</button>
						<button
							type="button"
							class="button primary"
							disabled={loading()}
							onClick={handleInvite}
						>
							{loading() ? "Accepting Invite..." : "Accept Invite"}
						</button>
					</div>
				</Match>
			</Switch>
		</Modal>
	);
};

// TODO: upload icon, set description when creating room
// TODO: list room templates instead of only "create room" button?
// TODO: split "create room" -> "create private room" and "create public room", have privacy-specific create forms?
