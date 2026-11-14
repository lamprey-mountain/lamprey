import { For, Show } from "solid-js";

const ROW_WIDTHS = [48, 62, 39, 55, 44, 58, 35, 51];

export function AuditLogSkeleton(props: { rows?: number; controls?: boolean }) {
	const rows = () => props.rows ?? 8;

	return (
		<div class="skeleton">
			<Show when={props.controls ?? true}>
				<div style="display:flex;gap:4px">
					<div>
						<div class="dim">&nbsp;</div>
						<div class="ghost" style="height:1.6em;width:76px" />
					</div>
				</div>
			</Show>
			<ul class="room-settings-audit-log">
				<For each={Array.from({ length: rows() }, (_, i) => i)}>
					{(i) => (
						<li
							style={{
								opacity: 1 - i / (rows() + 2),
								"pointer-events": "none",
							}}
						>
							<div class="info">
								<div style="display:flex;gap:4px">
									<div
										class="text ghost"
										style={`width:${ROW_WIDTHS[i % ROW_WIDTHS.length]}%`}
									/>
								</div>
								<div class="text ghost" style="height:.8em;width:72px" />
							</div>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
}
