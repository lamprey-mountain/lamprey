import { createResource, For, Show } from "solid-js";
import { useApi } from "@/api";
import { useModals } from "@/contexts/modal";

export function Search() {
	const api2 = useApi();
	const [, modalCtl] = useModals();

	const [stats, { refetch: refetchStats }] = createResource(() =>
		api2.client.http.GET("/api/v1/admin/search/stats").then((r) => r.data),
	);
	const [dlq, { refetch: refetchDlq }] = createResource(() =>
		api2.client.http.GET("/api/v1/admin/search/dlq").then((r) => r.data),
	);

	const reindexEverything = () => {
		modalCtl.confirm(
			"Are you sure you want to reindex EVERYTHING? This will delete all existing search data first.",
			(confirmed) => {
				if (confirmed) {
					api2.client.http
						.POST("/api/v1/admin/reindex-everything")
						.then(() => {
							modalCtl.alert("Reindexing queued.");
							refetchStats();
						})
						.catch((e) => modalCtl.alert(`Failed: ${e.message}`));
				}
			},
		);
	};

	const deleteDlq = (id: string) => {
		api2.client.http
			.DELETE("/api/v1/admin/search/dlq/{id}", {
				params: { path: { id } },
			})
			.then(() => refetchDlq());
	};

	const retryDlq = (id: string) => {
		api2.client.http
			.POST("/api/v1/admin/search/dlq/{id}/retry", {
				params: { path: { id } },
			})
			.then(() => {
				refetchDlq();
				refetchStats();
			});
	};

	return (
		<>
			<h2>Search Management</h2>
			<section class="section">
				<h3>Statistics</h3>
				<Show when={stats()} fallback={<p>Loading stats...</p>}>
					{(s) => (
						<ul class="stats-list">
							<li>
								<strong>Total Documents:</strong> {s().document_count}
							</li>
							<li>
								<strong>Index Size:</strong>{" "}
								{(s().index_size_bytes / 1024 / 1024).toFixed(2)} MB
							</li>
							<li>
								<strong>Backfill Queue:</strong> {s().backfill_queue_size} items
							</li>
						</ul>
					)}
				</Show>
				<div style="margin-top: 16px;">
					<button
						type="button"
						class="button danger"
						onClick={reindexEverything}
					>
						Reindex Everything
					</button>
				</div>
			</section>

			<section class="section" style="margin-top: 24px;">
				<h3>Dead Letter Queue (DLQ)</h3>
				<p class="dim">
					Entries that failed to be indexed and require manual intervention.
				</p>
				<table class="admin-table">
					<thead>
						<tr>
							<th>Type</th>
							<th>ID</th>
							<th>Error</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						<For
							each={dlq()?.items}
							fallback={
								<tr>
									<td colspan="4">No failures recorded.</td>
								</tr>
							}
						>
							{(entry) => (
								<tr>
									<td>{entry.entity_type}</td>
									<td class="dim">{entry.entity_id}</td>
									<td>{entry.error_message}</td>
									<td>
										<button
											type="button"
											class="button"
											onClick={[retryDlq, entry.id]}
										>
											retry
										</button>
										<button
											type="button"
											class="button danger"
											onClick={[deleteDlq, entry.id]}
										>
											delete
										</button>
									</td>
								</tr>
							)}
						</For>
					</tbody>
				</table>
			</section>
		</>
	);
}
