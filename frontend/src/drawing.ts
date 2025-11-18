import { Channel, Room, User } from "sdk";
import { generatePfp } from "./pfp.ts";
import { getThumbFromId } from "./media/util.tsx";
import { getColor } from "./colors.ts";

export const generateNotificationIcon = async (author: User, room?: Room) => {
	const c = new OffscreenCanvas(256, 256);
	const ctx = c.getContext("2d");
	if (!ctx) return null;

	try {
		const avatarUrl = author.avatar
			? getThumbFromId(author.avatar)
			: await generatePfp(author.id);
		const avatarImg = new Image();
		avatarImg.crossOrigin = "anonymous";
		avatarImg.src = avatarUrl;
		await new Promise((res, rej) => {
			avatarImg.onload = res;
			avatarImg.onerror = rej;
		});
		ctx.drawImage(avatarImg, 0, 0, 256, 256);
	} catch (e) {
		console.error("Failed to load author avatar for notification", e);
		// draw fallback
		ctx.fillStyle = "#36393f";
		ctx.fillRect(0, 0, 256, 256);
	}

	if (room?.icon) {
		try {
			const roomIconUrl = getThumbFromId(room.icon);
			const roomIconImg = new Image();
			roomIconImg.crossOrigin = "anonymous";
			roomIconImg.src = roomIconUrl;
			await new Promise((res, rej) => {
				roomIconImg.onload = res;
				roomIconImg.onerror = rej;
			});

			const roomIconSize = 80;
			const roomIconX = 256 - roomIconSize - 10;
			const roomIconY = 256 - roomIconSize - 10;

			ctx.save();
			ctx.beginPath();
			ctx.arc(
				roomIconX + roomIconSize / 2,
				roomIconY + roomIconSize / 2,
				roomIconSize / 2,
				0,
				Math.PI * 2,
			);
			ctx.closePath();
			ctx.clip();

			ctx.drawImage(
				roomIconImg,
				roomIconX,
				roomIconY,
				roomIconSize,
				roomIconSize,
			);

			ctx.restore();
		} catch (e) {
			console.error("Failed to load room icon for notification", e);
		}
	}

	return c.convertToBlob();
};

export const generateFavicon = async (
	mentionCount: number,
	icon?:
		| { type: "room"; room: Room }
		| { type: "channel"; channel: Channel & { recipients?: User[] } }
		| { type: "user"; user: User },
) => {
	const size = 64;
	const c = new OffscreenCanvas(size, size);
	const ctx = c.getContext("2d");
	if (!ctx) return null;

	ctx.clearRect(0, 0, size, size);

	let iconUrl: string | undefined;
	let iconBackgroundColor: string | undefined;

	if (icon) {
		switch (icon.type) {
			case "room":
				if (icon.room.icon) {
					iconUrl = getThumbFromId(icon.room.icon, size);
				}
				break;
			case "channel":
				if (icon.channel.icon) {
					iconUrl = getThumbFromId(icon.channel.icon, size);
				} else if (icon.channel.type === "Gdm") {
					iconBackgroundColor = getColor(icon.channel.id);
				}
				break;
			case "user":
				if (icon.user.avatar) {
					iconUrl = getThumbFromId(icon.user.avatar, size);
				} else {
					iconUrl = await generatePfp(icon.user.id);
				}
				break;
		}
	}

	ctx.save();
	ctx.beginPath();
	ctx.roundRect(0, 0, size, size, 12);
	ctx.clip();

	if (iconUrl) {
		try {
			const img = new Image();
			img.crossOrigin = "anonymous";
			img.src = iconUrl;
			await new Promise((res, rej) => {
				img.onload = res;
				img.onerror = rej;
			});
			ctx.drawImage(img, 0, 0, size, size);
		} catch (e) {
			console.error("Failed to load icon for favicon", e);
			iconUrl = undefined;
		}
	}

	if (!iconUrl) {
		if (iconBackgroundColor) {
			ctx.fillStyle = iconBackgroundColor;
			ctx.fillRect(0, 0, size, size);
		} else {
			// default icon
			ctx.fillStyle = "#36393f";
			ctx.fillRect(0, 0, size, size);
		}
	}
	ctx.restore();

	if (mentionCount > 0) {
		const circleRadius = 22;
		const circleX = size - circleRadius;
		const circleY = size - circleRadius;

		ctx.fillStyle = "oklch(54.03% 0.1759 13.16)";
		ctx.beginPath();
		ctx.arc(circleX, circleY, circleRadius, 0, 2 * Math.PI);
		ctx.fill();

		ctx.fillStyle = "white";
		ctx.font = "bold 32px sans-serif";
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		// TODO: show full mention count, use rounded rectangle instead of circle
		const text = mentionCount > 9 ? "9+" : mentionCount.toString();
		ctx.fillText(text, circleX, circleY, circleRadius * 2 - 4);
	}

	return c.convertToBlob();
};
