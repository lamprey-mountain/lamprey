import { Room } from "sdk";
import { createSignal, useContext } from "solid-js";
import { chatctx } from "./context.ts";

export const RoomSettings = (props: { room: Room }) => {
  const [currentInvite, setCurrentInvite] = createSignal();
  const ctx = useContext(chatctx)!;
  
  const setName = () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			name: prompt("name?")
		})
  }
  
  const setDescription = () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			description: prompt("description?")
		})
  }

  const createInvite = async () => {
		const invite = await ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/invites`, {});
		console.log(invite);
		setCurrentInvite(invite);
  }
  // qFMhEkFrSaWP_3mvv5LDN
  
  return (
		<div class="flex-1 bg-bg2 text-fg2">
  		room settings<br />
  		{props.room.data.description}<br />
		  <button onClick={setName}>set name</button><br />
		  <button onClick={setDescription}>set description</button><br />
		  <button onClick={createInvite}>create invite</button><br />
	    last invite code: <code>{currentInvite()?.code}</code><br />
		</div>
  )
}
