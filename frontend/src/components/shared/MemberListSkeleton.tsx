export const MemberListSkeleton = () => {
	return (
		<div class="member-list skeleton">
			<div class="member-group">
				<div class="ghost text" style="width: 30%" />
			</div>
			{Array.from({ length: 10 }).map((_, index) => (
				<div
					class="menu-user"
					style={{
						"pointer-events": "none",
						opacity: 1 - index / 10,
					}}
				>
					<div class="inner">
						<div class="avatar">
							<div class="inner ghost"> </div>
						</div>
						<span class="text">
							<div
								class="name text ghost"
								style="width: 60%; border-radius: 2px"
							/>
							<div
								class="status-message text ghost"
								style="width: 40%; border-radius: 2px"
							/>
						</span>
					</div>
				</div>
			))}
		</div>
	);
};
