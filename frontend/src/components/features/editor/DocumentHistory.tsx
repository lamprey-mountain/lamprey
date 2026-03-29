import type { Channel, HistoryPagination } from "sdk";
import { createEffect, createSignal, For, on, Show } from "solid-js";
import { useApi2 } from "@/api";
import { Time } from "../../../atoms/Time.tsx";
import { Avatar } from "../../../avatar/UserAvatar.tsx";
import { useChannel } from "../../../contexts/channel.tsx";

type ChangesetSelection = {
	start_seq: number;
	end_seq: number;
};

type DocumentHistoryProps = {
	channel: Channel;
	branchId: string;
	isOpen: boolean;
	onSelectChangeset: (changeset: ChangesetSelection | null) => void;
	onHoverChangeset: (changeset: ChangesetSelection | null) => void;
	selectedSeq: ChangesetSelection | null;
};

const AvatarWithTooltip = (props: { user: any; name: string }) => {
	let wrap: HTMLDivElement | undefined;
	let tipEl: HTMLDivElement | undefined;
	const [visible, setVisible] = createSignal(false);
	const [position, setPosition] = createSignal({ x: 0, y: 0 });

	const updatePosition = () => {
		if (!wrap) return;
		const rect = wrap.getBoundingClientRect();
		setPosition({ x: rect.left, y: rect.top - 40 });
	};

	const showTip = () => {
		updatePosition();
		setVisible(true);
	};

	const hideTip = () => setVisible(false);

	createEffect(() => {
		if (visible()) {
			window.addEventListener("scroll", updatePosition, true);
			window.addEventListener("resize", updatePosition);
			return () => {
				window.removeEventListener("scroll", updatePosition, true);
				window.removeEventListener("resize", updatePosition);
			};
		}
	});

	return (
		<>
			<div
				ref={wrap!}
				onMouseEnter={showTip}
				onMouseLeave={hideTip}
				style="display: inline-block"
			>
				<Avatar animate={false} user={props.user} pad={2} />
			</div>
			<Show when={visible()}>
				<div
					ref={tipEl!}
					class="tooltip"
					style={{
						position: "fixed",
						left: `${position().x}px`,
						top: `${position().y}px`,
					}}
				>
					<div class="base"></div>
					<div class="inner">{props.name}</div>
				</div>
			</Show>
		</>
	);
};

export const DocumentHistory = (props: DocumentHistoryProps) => {
	const api2 = useApi2();
	const [, setCh] = useChannel()!;
	const [history, setHistory] = createSignal<HistoryPagination | null>(null);
	const [loading, setLoading] = createSignal(false);
	const [error, setError] = createSignal<string | null>(null);

	const loadHistory = async () => {
		setLoading(true);
		setError(null);
		try {
			const data = await api2.documents.history(
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
					<div
						class="history-list"
						onMouseLeave={() => props.onHoverChangeset(null)}
					>
						<For each={history()!.changesets}>
							{(changeset) => {
								const isSelected =
									props.selectedSeq !== null &&
									props.selectedSeq.start_seq === changeset.start_seq &&
									props.selectedSeq.end_seq === changeset.end_seq;
								return (
									<div
										class="history-item"
										classList={{ selected: isSelected }}
										onClick={() => {
											if (isSelected) {
												props.onSelectChangeset(null);
											} else {
												props.onSelectChangeset({
													start_seq: changeset.start_seq,
													end_seq: changeset.end_seq,
												});
											}
										}}
										onMouseEnter={() =>
											props.onHoverChangeset({
												start_seq: changeset.start_seq,
												end_seq: changeset.end_seq,
											})
										}
									>
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
														<For each={visibleAuthors}>
															{(authorId) => {
																const user = history()!.users.find(
																	(u) => u.id === authorId,
																);
																const userName = user?.name ?? authorId;
																return (
																	<AvatarWithTooltip
																		user={user}
																		name={userName}
																	/>
																);
															}}
														</For>
														{remainingCount > 0 && (
															<div
																class="avatar avatar-remaining"
																title={`${remainingCount} more author(s)`}
															>
																+{remainingCount}
															</div>
														)}
													</>
												);
											})()}
										</div>
									</div>
								);
							}}
						</For>
					</div>
				</Show>
				<Show when={!loading() && !error() && !history()}>
					<div>No history available</div>
				</Show>
			</div>
		</div>
	);
};
