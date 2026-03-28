import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import type { ReactionCount, ReactionKey, User } from "sdk";
import { Modal } from "./mod";
import { useApi2, useMessages2, useUsers2 } from "@/api";
import { Avatar } from "../User";
import { renderReactionKey } from "../emoji";

interface ModalReactionsProps {
	channel_id: string;
	message_id: string;
}

const reactionKeyToParam = (key: ReactionKey): string => {
	if (key.type === "Text") {
		return `t:${key.content}`;
	} else if (key.type === "Custom") {
		return `c:${key.id}`;
	}
	return "";
};

export const ModalReactions = (props: ModalReactionsProps) => {
	const api2 = useApi2();
	const users2 = useUsers2();
	const messagesService = useMessages2();
	const message = messagesService.use(() => props.message_id);

	const reactions = () => message()?.reactions ?? [];
	const [selectedReaction, setSelectedReaction] = createSignal<
		ReactionKey | null
	>(
		null,
	);

	createEffect(() => {
		const r = reactions();
		if (r.length > 0 && selectedReaction() === null) {
			setSelectedReaction(r[0].key);
		}
	});

	const [reactors, setReactors] = createSignal<{ user_id: string }[]>([]);
	const [afterCursor, setAfterCursor] = createSignal<string | undefined>();
	const [hasMore, setHasMore] = createSignal(false);
	let sentinel: HTMLDivElement | undefined;

	const [pageData] = createResource(
		() => ({ reaction: selectedReaction(), after: afterCursor() }),
		async ({ reaction, after }) => {
			if (!reaction) return null;
			return await api2.reactions.list(
				props.channel_id,
				props.message_id,
				reactionKeyToParam(reaction),
				{ limit: 50, after: after },
			);
		},
	);

	createEffect(() => {
		const data = pageData();
		if (pageData.loading || !data) return;

		if (afterCursor() === undefined) {
			setReactors(data.items);
		} else {
			setReactors((p) => [...p, ...data.items]);
		}
		setHasMore(data.has_more);
	});

	createEffect(() => {
		selectedReaction();
		setReactors([]);
		setAfterCursor(undefined);
	});

	onMount(() => {
		const observer = new IntersectionObserver((entries) => {
			if (entries[0].isIntersecting && hasMore() && !pageData.loading) {
				const lastReactor = reactors().at(-1);
				if (lastReactor) {
					setAfterCursor(lastReactor.user_id);
				}
			}
		});

		if (sentinel) {
			observer.observe(sentinel);
		}

		onCleanup(() => {
			if (sentinel) {
				observer.unobserve(sentinel);
			}
		});
	});

	return (
		<Modal>
			<div class="reactions-modal">
				<div class="reactions">
					<For each={reactions()}>
						{(reaction) => {
							const key = reaction.key;
							return (
								<button
									onClick={() => setSelectedReaction(key)}
									data-selected={selectedReaction() === key}
								>
									<div
										style="display:contents"
										innerHTML={renderReactionKey(key)}
									/>
									<div>{reaction.count}</div>
								</button>
							);
						}}
					</For>
				</div>
				<div class="users">
					<For each={reactors()}>
						{(reactor) => {
							const user = users2.use(() => reactor.user_id);
							return (
								<div class="user">
									<Avatar user={user()} />
									<div>{user()?.name ?? "..."}</div>
								</div>
							);
						}}
					</For>
					<Show when={pageData.loading}>
						<div>Loading...</div>
					</Show>
					<div ref={sentinel} />
				</div>
			</div>
		</Modal>
	);
};
