import { User } from "sdk";

export const UserPage = (p: { user: User }) => {
	return (
		<div class="user-profile page">
			<div class="banner"></div>
			<header>
				<div class="avatar">
				</div>
				<span class="name">
					{p.user.name}
				</span>
				<div class="menu">
					<button>message</button>
					<button>friend</button>
				</div>
			</header>
			<div style="padding:8px">
				<div style="padding: 8px; background: #111">
					{p.user.description}
				</div>
				<br />
				<div>
					user id: <code>{p.user.id}</code>
				</div>
				<br />
				{p.user.status.type}
				{p.user.type}
				{p.user.state}
				<br />
			</div>
		</div>
	);
};
