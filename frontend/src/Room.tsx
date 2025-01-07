import { createResource, For, Show, useContext } from "solid-js";
import { MemberT, Pagination, RoomT, ThreadT } from "./types.ts";
import { chatctx } from "./context.ts";
import { Message } from "./Messages.tsx";
import { getTimestampFromUUID } from "sdk";

const CLASS_BUTTON = "px-1 bg-bg3 hover:bg-bg4 my-0.5";

export const RoomHome = (props: { room: RoomT }) => {
  const ctx = useContext(chatctx)!;
	const room_id = props.room.id;
	
	async function createThread(room_id: string) {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "name?" });
		ctx.client.http("POST", `/api/v1/rooms/${room_id}/threads`, {
			name
		});
	}
	
	async function leaveRoom(room_id: string) {
  	if (!await ctx.dispatch({ do: "modal.confirm", text: "are you sure you want to leave?" })) return;
		ctx.client.http("DELETE", `/api/v1/rooms/${room_id}/members/@self`);
	}
	
  const [threads, { refetch: fetchThreads }] = createResource<Pagination<ThreadT> & { room_id: string }, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.room_id !== room_id) value = undefined;
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
  	const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/threads?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  		room_id,
  	};
  });
	
  // <div class="date"><Time ts={props.thread.baseEvent.originTs} /></div>
  // TODO: use actual links instead of css styled divs
	return (
		<div class="flex-1 bg-bg2 text-fg2 p-4 overflow-y-auto">
			<h2 class="text-xl">{props.room.name}</h2>
			<p>{props.room.description}</p>
			<button class={CLASS_BUTTON} onClick={() => createThread(room_id)}>create thread</button><br />
			<button class={CLASS_BUTTON} onClick={() => leaveRoom(room_id)}>leave room</button><br />
			<button class={CLASS_BUTTON} onClick={() => ctx.dispatch({ do: "setView", to: { view: "room-settings", room: props.room }})}>settings</button><br />
			<br />
			<ul>
	    	<For each={threads()?.items}>{thread => (
	      	<li>
	      	<article class="contain-content bg-bg3 my-[8px] border-[1px] border-sep [contain:content] max-w-[800px]">
		      	<header
			      	class="flex flex-col px-[8px] py-[4px] cursor-pointer bg-bg3 border-b-[1px] border-b-sep"
			      	onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room: props.room, thread }})}
		      	>
			        <div class="flex items-center gap-[8px] leading-none">
			          <div class="bg-bg4 h-[16px] w-[16px] rounded-full"></div>
			          <div class="truncate text-lg flex-1">{thread.name}</div>
			          <div class="text-fg2">Created at {getTimestampFromUUID(thread.id).toDateString()}</div>
			        </div>
			        <div class="self-start mt-[8px] text-fg2 cursor-pointer hover:text-fg1 hover:underline" onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room: props.room, thread }})}>
			          message count &bull; last msg {getTimestampFromUUID(thread.id).toDateString()}
		          	<Show when={thread.description}>
		          		<br />
				          {thread.description}
	          		</Show>
			        </div>
		      	</header>
	  	      <Show when={true}>
			        <div class="preview">
			          <For each={[]}>
			            {(ev) => <Message message={ev} />}
			          </For>
			          <details class="p-1 cursor-pointer">
				          <summary>json data</summary>
				          <pre>
						      	{JSON.stringify(thread, null, 4)}
				          </pre>
			          </details>
			        </div>
			      </Show>
			      <Show when={false}>
			        <footer class="cursor-pointer text-center bg-gradient-to-t from-bg1/50 fixed bottom-0 left-0 w-full py-[4px] px-[8px]">
			          message.remaining
			        </footer>
			      </Show>
	      	</article>
	    		</li>
	    	)}</For>
			</ul>
			<button class={CLASS_BUTTON} onClick={fetchThreads}>load more</button><br />
		</div>
	);
}
