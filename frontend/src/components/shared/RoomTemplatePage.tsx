import { type RouteSectionProps, useNavigate } from "@solidjs/router";
import {
	createMemo,
	createResource,
	createSignal,
	Match,
	type ParentProps,
	Show,
	Switch,
} from "solid-js";
import type { ApiError, RoomTemplate, RoomTemplateChannel } from "ts-sdk";
import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { ApiErrorDisplay, isApiError } from "@/atoms/ApiErrorDisplay";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { Icon } from "@/atoms/Icon";
import { ChannelIconRoom } from "@/avatar/ChannelIcon";
import { Avatar, Avatar2 } from "@/avatar/UserAvatar";
import { icQuestion } from "@/utils/icons";
import { Title } from "./Title";
import { UserDisplayName } from "./User";

type RoomTemplateResult =
	| { status: "loading" }
	| { status: "loaded"; template: RoomTemplate }
	| { status: "apiError"; err: ApiError }
	| { status: "platformError"; err: unknown };

export const RoomTemplatePage = (p: ParentProps<RouteSectionProps>) => {
	const { t } = useCtx();
	const api = useApi();

	const [templateResource] = createResource<RoomTemplateResult, string | null>(
		() => p.params.template_id ?? null,
		async (code: string | null) => {
			if (!code)
				throw new Error("router should never route without a template id");
			try {
				const { data, error } = await api.client.http.GET(
					"/api/v1/room-template/{code}",
					{ params: { path: { code } } },
				);
				if (error) throw error;
				const template = data as unknown as RoomTemplate;
				api.users.upsert(template.creator);
				return { status: "loaded", template };
			} catch (err) {
				if (isApiError(err)) {
					return { status: "apiError", err };
				}
				return { status: "platformError", err };
			}
		},
		{ initialValue: { status: "loading" } },
	);

	const templateName = () => {
		const t = templateResource();
		switch (t?.status) {
			case "loaded":
				return t.template.name;
			case "apiError":
			case "platformError":
				return "???";
			default:
				return "...";
		}
	};

	function matches<T extends RoomTemplateResult["status"]>(
		ty: T,
	): (RoomTemplateResult & { status: T }) | false {
		const r = templateResource();
		if (r?.status === ty) {
			return r as RoomTemplateResult & { status: T };
		} else {
			return false;
		}
	}

	return (
		<>
			<header class="chat-header">
				<div class="channel-icon">
					{/* TODO: better icon */}
					<Icon src={icQuestion} />
				</div>
				<div class="name">
					<h3 class="name-text">{templateName()}</h3>
				</div>
				<div class="spacer"></div>
			</header>
			<div class="room-template-page-wrapper">
				<Switch>
					<Match when={matches("loading")}>
						<div>loading...</div>
					</Match>
					<Match when={matches("apiError")}>
						{(err) => (
							<div style="display:grid;place-items:center;height:100%">
								<ApiErrorDisplay error={err().err} />
							</div>
						)}
					</Match>
					<Match when={matches("platformError")}>
						<div>internal error</div>
					</Match>
					<Match when={matches("loaded")}>
						{(t) => (
							<>
								<Title title={t().template.name} />
								<RoomTemplateInner template={t().template} />
							</>
						)}
					</Match>
				</Switch>
			</div>
		</>
	);
};

export const RoomTemplateInner = (props: { template: RoomTemplate }) => {
	const nav = useNavigate();
	const api = useApi();
	const sourceRoom = api.rooms.use(
		() => props.template.source_room_id ?? undefined,
	);

	const [loading, setLoading] = createSignal(false);
	const [roomName, setRoomName] = createSignal(props.template.name);
	const [isPublic, setIsPublic] = createSignal(false);

	const categorizedChannels = createMemo(() => {
		const t = props.template;
		const channels = t.snapshot.channels;
		const categories = new Map(
			channels
				.filter((c) => c.type === "Category")
				.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
				.map((c) => [
					c.id,
					{ category: c, children: [] as RoomTemplateChannel[] },
				]),
		);
		const uncategorized: RoomTemplateChannel[] = [];

		channels.forEach((chan) => {
			if (chan.type === "Category") return;

			const category = chan.parent_id && categories.get(chan.parent_id);
			if (category) {
				category.children.push(chan);
			} else {
				uncategorized.push(chan);
			}
		});

		categories.forEach((cat) => {
			cat.children.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
		});

		return {
			categories: Array.from(categories.values()),
			uncategorized,
		};
	});

	const handleCreate = async () => {
		if (loading()) return;
		const name = roomName().trim();
		if (!name) return; // TODO: handle invalid input
		setLoading(true);
		try {
			// TODO: add an endpoint to actually use a room template
			// const { data, error } = await api.client.http.POST(
			// 	"/api/v1/???",
			// 	{
			// 		params: { path: { code: props.template.code } },
			// 		body: { name, public: isPublic() },
			// 	},
			// );
			// if (error) throw error;
			// nav(`/room/${data.id}`);
		} catch (err) {
			console.error("failed to create room from template", err);
			// TODO: show error to user
		} finally {
			setLoading(false);
		}
	};

	const cancel = () => {
		nav("/");
	};

	return (
		<div class="room-template-page">
			<div class="main">
				<header class="header">
					<h3 class="dim">room template</h3>
					<h1>{props.template.name}</h1>
					<p>{props.template.description}</p>
				</header>
				<div class="creator">
					<span class="dim">created by</span>
					<Avatar user={props.template.creator} />
					<UserDisplayName user_id={props.template.creator.id} onClick />
					<Show when={sourceRoom()}>
						{(r) => (
							<>
								{" "}
								<span class="dim">from</span>{" "}
								<a href={`/room/${r().id}`}>{r().name}</a>
							</>
						)}
					</Show>
				</div>
				{/* TODO: dedupe form with ModalRoomCreateOrJoin */}
				<form
					class="form"
					onSubmit={(e) => {
						e.preventDefault();
						handleCreate();
					}}
				>
					<label>
						<h3 class="dim">room name</h3>
						<input
							type="text"
							value={roomName()}
							onInput={(e) => setRoomName(e.currentTarget.value)}
							placeholder="room name"
							required
							disabled={loading()}
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

					<menu class="menu">
						<button
							class="button link"
							onClick={cancel}
							type="button"
							disabled={loading()}
						>
							nevermind
						</button>
						<button class="button primary" type="submit" disabled={loading()}>
							{loading() ? "Creating Room..." : "use this template"}
						</button>
					</menu>
				</form>
			</div>
			<aside class="details">
				<h3 class="dim">channels</h3>
				<Show when={categorizedChannels()}>
					{(data) => (
						<ul>
							{data().uncategorized.map((c) => (
								<li class="channel">
									<ChannelIconRoom
										id={c.id}
										type={c.type ?? "Text"} // NOTE: type should never be undefined
										nsfw={c.nsfw}
									/>

									{c.name}
								</li>
							))}
							{data().categories.map((cat) => (
								<li>
									<strong>{cat.category.name}</strong>
									<ul>
										{cat.children.map((c) => (
											<li class="channel">
												<ChannelIconRoom
													id={c.id}
													type={c.type ?? "Text"} // NOTE: type should never be undefined
													nsfw={c.nsfw}
												/>

												{c.name}
											</li>
										))}
									</ul>
								</li>
							))}
						</ul>
					)}
				</Show>
				<br />
				<h3 class="dim">roles</h3>
				<ul>
					{props.template.snapshot.roles.map((r) => (
						<li class="channel">{r.name}</li>
					))}
				</ul>
			</aside>
		</div>
	);
};
