import { getOwner, runWithOwner, VoidComponent } from "solid-js";
import { render } from "solid-js/web";
import { getEmojiUrl } from "../media/util.tsx";
import { type Api } from "../api.tsx";

export const createNodeViews = () => {
	const owner = getOwner();

	return function <T extends Record<string, any>>(
		propsFn: (node: any) => T,
		Component: VoidComponent<T>,
	) {
		return (node: any) => {
			const dom = document.createElement("span");
			dom.classList.add("mention");

			const dispose = render(
				() => runWithOwner(owner, () => <Component {...propsFn(node)} />),
				dom,
			);

			return {
				dom,
				destroy: () => dispose(),
			};
		};
	};
};

export const createEditorNodeViews = (
	api: Api,
	opts?: {
		currentChannelId?: () => string;
	},
) => {
	const nv = createNodeViews();

	return () => ({
		mention: nv(
			(n) => ({ id: n.attrs.user, name: n.attrs.name }),
			(props) => {
				const getUserId = () => props.id;
				if (opts?.currentChannelId) {
					const channel = api.channels.fetch(() => opts.currentChannelId!());
					const user = api.users.fetch(getUserId);
					const roomMember = api.room_members.fetch(
						() => channel()?.room_id!,
						getUserId,
					);
					const name = () => {
						const id = getUserId();
						if (!id) return "..."; // Placeholder while loading/missing
						return roomMember()?.override_name ?? user()?.name ?? id;
					};
					return <span class="mention-user">@{name()}</span>;
				} else {
					const user = api.users.fetch(getUserId);
					return (
						<span class="mention-user">
							@{user()?.name ?? getUserId() ?? "..."}
						</span>
					);
				}
			},
		),
		mentionChannel: nv(
			(n) => ({ id: n.attrs.channel, name: n.attrs.name }),
			(props) => {
				const getChannelId = () => props.id;
				const channel = api.channels.fetch(getChannelId);
				const name = () => channel()?.name ?? getChannelId() ?? "...";
				return <span class="mention-channel">#{name()}</span>;
			},
		),
		mentionRole: nv(
			(n) => ({ id: n.attrs.role, name: n.attrs.name }),
			(props) => {
				const getRoleId = () => props.id;
				const role = () => api.roles.cache.get(getRoleId());
				const name = () => role()?.name ?? getRoleId() ?? "...";
				return <span class="mention-role">@{name()}</span>;
			},
		),
		mentionEveryone: nv(
			() => ({}),
			() => {
				return <span class="mention-everyone">@everyone</span>;
			},
		),
		emoji: nv(
			(n) => ({ id: n.attrs.id, name: n.attrs.name }),
			(props) => {
				const url = getEmojiUrl(props.id);
				return (
					<img
						class="emoji"
						src={url}
						alt={`:${props.name ?? ""}:`}
						title={`:${props.name ?? ""}:`}
					/>
				);
			},
		),
	});
};
