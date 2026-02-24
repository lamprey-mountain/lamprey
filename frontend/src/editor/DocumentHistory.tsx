import { createEffect, createSignal, on, Show } from "solid-js";
import type { Channel } from "sdk";
import { useApi } from "../api.tsx";
import type { HistoryPagination } from "sdk";
import { Time } from "../Time.tsx";
import { useChannel } from "../contexts/channel.tsx";
import { Avatar } from "../avatar/UserAvatar.tsx";
import { createTooltip } from "../Tooltip.tsx";

type DocumentHistoryProps = {
	channel: Channel;
	branchId: string;
	isOpen: boolean;
};

export const DocumentHistory = (props: DocumentHistoryProps) => {
	const api = useApi();
	const [, setCh] = useChannel()!;
	const [history, setHistory] = createSignal<HistoryPagination | null>(null);
	const [loading, setLoading] = createSignal(false);
	const [error, setError] = createSignal<string | null>(null);

	const loadHistory = async () => {
		setLoading(true);
		setError(null);
		try {
			const data = await api.documents.history(
				props.channel.id,
				props.branchId,
				{
					limit: 50,
					by_author: false,
					by_changes: 100,
					by_tag: true,
					by_time: 60 * 5,
				},
			);
			setHistory(data);
		} catch (e) {
			console.error("Failed to load document history:", e);
			setError("Failed to load history");
		} finally {
			setLoading(false);
		}
	};

	createEffect(
		on(
			() => ({
				isOpen: props.isOpen,
				branchId: props.branchId,
				channelId: props.channel.id,
			}),
			() => {
				if (props.isOpen) {
					loadHistory();
				}
			},
		),
	);

	return (
		<div class="document-history">
			<header class="document-history-header">
				<h3>History</h3>
				<button onClick={() => setCh("history_view", false)}>Close</button>
			</header>
			<div class="document-history-content">
				<Show when={loading()}>
					<div>Loading history...</div>
				</Show>
				<Show when={error()}>
					<div class="error">{error()}</div>
				</Show>
				<Show when={!loading() && !error() && history()}>
					<div class="history-list">
						{history()!.changesets.map((changeset) => (
							<div class="history-item">
								<div class="history-item-header">
									<Time date={new Date(changeset.start_time)} />
									<div style="flex:1"></div>
									<div class="history-item-stat history-item-stat-added">
										+{changeset.stat_added}
									</div>
									<div class="history-item-stat history-item-stat-removed">
										-{changeset.stat_removed}
									</div>
								</div>
								<div class="history-item-authors">
									{(() => {
										const visibleAuthors = changeset.authors.slice(0, 5);
										const remainingCount = changeset.authors.length - 5;
										return (
											<>
												{visibleAuthors.map((authorId) => {
													const user = history()!.users.find((u) =>
														u.id === authorId
													);
													const tip = createTooltip({
														tip: () => user?.name ?? authorId,
													});
													return (
														<div ref={tip.content}>
															<Avatar animate={false} user={user} pad={2} />
														</div>
													);
												})}
												{remainingCount > 0 && (
													<div class="avatar avatar-remaining">
														+{remainingCount}
													</div>
												)}
											</>
										);
									})()}
								</div>
							</div>
						))}
					</div>
				</Show>
				<Show when={!loading() && !error() && !history()}>
					<div>No history available</div>
				</Show>
			</div>
		</div>
	);
};
