import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import type { User } from "sdk";
import { Modal } from "./mod";
import { useApi } from "../api";
import { Avatar } from "../User";

interface ModalReactionsProps {
	channel_id: string;
	message_id: string;
}

export const ModalReactions = (props: ModalReactionsProps) => {
	const api = useApi();
	const message = api.messages.fetch(
		() => props.channel_id,
		() => props.message_id,
	);

	const reactions = () => message()?.reactions ?? [];
	const [selectedReaction, setSelectedReaction] = createSignal<string | null>(
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
			return await api.reactions.list(
				props.channel_id,
				props.message_id,
				reaction,
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
						{(reaction) => (
							<button
								onClick={() => setSelectedReaction(reaction.key)}
								data-selected={selectedReaction() === reaction.key}
							>
								<div>{reaction.key}</div>
								<div>{reaction.count}</div>
							</button>
						)}
					</For>
				</div>
				<div class="users">
					<For each={reactors()}>
						{(reactor) => {
							const user = api.users.fetch(() => reactor.user_id);
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
