import { ParentComponent } from "solid-js";

const FORUM_CSS = `
  .comment {
    border-left: solid var(--borders) 1px;
    margin-left: -1px;
    margin-bottom: -1px;
    
    &.replied {
      border-left: solid var(--color-accent) 1px;
    }
    
    & > header {
      display: flex;
      gap: 8px;
      background: var(--bg-secondary);
      white-space: nowrap;

      & > .collapse {
        min-width: 24px;
        border: solid var(--borders) 1px;
        border-left: none;
        border-radius: 0;
        background: none;
        font-family: var(--font-mono);

        &:hover {
          background: var(--bg-light);
          text-decoration: none;
        }
      }

      & > .childCount {
        color: var(--fg-dimmed);
      }

      & > .author > .green {
        color: #77c57d;
      }

      & > time {
        color: var(--fg-dimmed);
      }

      & > .summary {
        color: var(--fg-dimmed);
        font-style: italic;
        overflow: hidden;
        text-overflow: ellipsis;

        &::before {
          content: "-";
          margin-right: 8px;
        }
      }
    }
    
    &.collapsed > header {
      background: none;
    }

    & > .content {
      padding: 8px;
    }

    & > menu {
      display: flex;
      gap: 8px;
      padding: 0 8px;
      padding-bottom: 12px;
      
      & > button {
        background: none;
        border: none;
        padding: 0;
        font-size: .9rem;
        color: var(--fg-dimmed);
      }
    }

    & > .children {
      list-style: none;
      margin-left: 24px;
    }
  }
`;

const Forum = () => (
	<ShadowRoot>
		<style>{FORUM_CSS}</style>
	</ShadowRoot>
);

// const Comment = ({ state, event, comments }) => {
//   const [collapsed, setCollapsed] = createSignal(false);
//   const [replied, setReplied] = createSignal(false);
//   const isFromOp = event.pubkey === comments.rootEvent?.pubkey;

//   const [author] = createResource(() => event.pubkey, async (pubkey) => {
//     // replace with actual author-fetching logic
//     return await fetchAuthor(pubkey);
//   });

//   const countChildren = () => {
//     return comments.events.filter(e => e.tags.some(tag => tag[0] === "e" && tag[1] === event.id)).length;
//   };

//   const children = () => {
//     return comments.events.filter(e => e.tags.some(tag => tag[0] === "e" && tag[1] === event.id));
//   };

//   const handleCollapseToggle = () => {
//     state.curry(collapsed() ? "expand" : "collapse", event.id);
//     setCollapsed(!collapsed());
//   };

//   const handleReplyToggle = () => {
//     state.curry("reply", replied() ? null : event.id);
//     setReplied(!replied());
//   };

// 	return (
// 		<div
// 			class="comment"
// 			classList={{
// 				replied: replied(),
// 				fromop: isFromOp,
// 				collapsed: collapsed(),
// 			}}
// 		>
// 			<header>
// 				<button class="collapse" onClick={handleCollapseToggle}>
// 					{collapsed() ? "+" : "-"}
// 				</button>

// 				<Show when={collapsed()}>
// 					<span class="childCount">[{countChildren()}]</span>
// 				</Show>
// 				<div class="author">
// 					<Show
// 						when={author()}
// 						fallback={<i>loading...</i>}
// 					>
// 						{(authorData) => {
// 							const name = authorData?.getContent?.()?.name;
// 							const recent = authorData?.origin_ts <
// 								Date.now() + 1000 * 60 * 60 * 24 * 7;
// 							return name
// 								? (
// 									isFromOp
// 										? <b>{name}</b>
// 										: recent
// 										? <span class="green">{name}</span>
// 										: name
// 								)
// 								: <i>anonymous</i>;
// 						}}
// 					</Show>
// 				</div>
// 				<time datetime={new Date(event.origin_ts).toISOString()}>
// 					{timeAgo(event.origin_ts)}
// 				</time>
// 				<Show when={collapsed()}>
// 					<div class="summary">{event.content.body}</div>
// 				</Show>
// 			</header>

// 			<Show when={!collapsed()}>
// 				<div class="content">{event.content.body}</div>
// 				<menu>
// 					<button onClick={handleReplyToggle}>
// 						{replied() ? "deselect" : "reply"}
// 					</button>
// 					<button onClick={() => navigator.clipboard.writeText(event.id)}>
// 						share
// 					</button>
// 				</menu>
// 				<Show when={children().length}>
// 					<ul class="children">
// 						<For each={children()}>
// 							{(child) => (
// 								<li>
// 									<Comment state={state} event={child} comments={comments} />
// 								</li>
// 							)}
// 						</For>
// 					</ul>
// 				</Show>
// 			</Show>
// 		</div>
// 	);
// };

// <script lang="ts">
//   import { api } from "../../lib/api";
//   import type { Event } from "../../lib/api";
//   import Comments from "./Comments.svelte";
//   import { filterRels, query } from "../../lib/util";
//   import { Reduxer } from "../../lib/reduxer";
//   import { tick } from "svelte";
//   export let options = new URLSearchParams();
//   export let bucket: Event;
//   $: forumId = filterRels(bucket, "in")[0];
//   let commentBox: HTMLFormElement;

//   const state = new Reduxer({
//     replyId: null as null | string,
//     opId: bucket.sender,
//     collapsed: new Set(),
//   }, {
//     reply(_state, replyId: string | null) {
//       if (replyId) {
//         tick().then(() => commentBox?.scrollIntoView({ behavior: "smooth", block: "center" }));
//       }
//       return { replyId };
//     },
//     collapse(state, commentId: string) {
//       state.collapsed.add(commentId);
//       return state;
//     },
//     expand(state, commentId: string) {
//       state.collapsed.delete(commentId);
//       return state;
//     },
//     expandAll(state) {
//       state.collapsed.clear();
//       return state;
//     },
//     collapseTopLevel(state) {
//       const collapsed = [...state.collapsed];
//       for (const comment of $comments) {
//         if (topLevelComments.find(i => i.id === filterRels(comment, "comment")[0])) {
//           collapsed.push(comment.id);
//         }
//       }
//       return { ...state, collapsed: new Set(collapsed) };
//     },
//   });

//   const comments = query({
//     refs: [bucket.id],
//     relations: [["l.forum.comment", "comment"]],
//   }, event => event.type === "l.forum.comment");
//   $: topLevelComments = $comments
//     .filter(comment => filterRels(comment, "comment").indexOf(bucket.id) !== -1)
//     .sort((a, b) => a.origin_ts - b.origin_ts);

//   let commentBody: string;

//   async function handleComment(e) {
//     await api.createEvent("l.forum.comment", {
//       body: commentBody || undefined,
//     }, {
//       [$state.replyId ?? bucket.id]: { type: "comment" },
//     });
//     commentBody = "";
//     state.do("reply", null);
//   }
// </script>
// <div class="wrapper">
//   <div>
//     <a href="/#/{forumId}">back</a>
//   </div>
//   <article class="post">
//     <h1 style="font-weight: bold">{bucket.content.title || "no title"}</h1>
//     <p>{bucket.content.body || "no body"}</p>
//   </article>
//   <hr />
//   {$comments.length} comments - <button on:click={state.curry("collapseTopLevel")}>collapse</button> <button on:click={state.curry("expandAll")}>expand</button>
//   <ul class="comments">
//   {#each topLevelComments as event (event.id)}
//   	<li class="toplevel"><Comments {state} comments={$comments} {event} /></li>
//   {:else}
//     <em>no comments</em>
//   {/each}
//   </ul>
//   <hr />
//   <form on:submit|preventDefault={handleComment} bind:this={commentBox}>
//     <table>
//       <tr><td><em>new comment</em></td></tr>
//       {#if $state.replyId}
//       <tr><td>reply:</td><td><button on:click={state.curry("reply", null)}>deselect</button></td></tr>
//       {/if}
//       <tr><td>comment:</td><td><textarea bind:value={commentBody} placeholder="say something nice"></textarea></td></tr>
//       <tr><td></td><td><input type="submit" value="post"></td></tr>
//     </table>
//   </form>
// </div>
// <style lang="scss">
//   .wrapper {
//     margin: 1em;
//   }

//   .post {
//     margin: 1em 0;
//   }

//   .comments {
//     list-style: none;
//     margin-left: 0;

//     & > .toplevel {
//       margin-top: 16px;
//     }
//   }

//   hr {
//     border-top: none;
//     border-bottom: solid var(--borders) 1px;
//     margin: 8px -1em;
//   }

//   em {
//     font-style: italic;
//   }
// </style>

/**
 * A declarative shadow root component
 *
 * Hooks into SolidJS' Portal's `useShadow` prop
 * to handle shadow DOM and the component lifecycle
 */
export const ShadowRoot: ParentComponent = (props) => {
	let div: HTMLDivElement;
	return (
		<div ref={div!}>
			<Portal mount={div!} useShadow={true}>
				{props.children}
			</Portal>
		</div>
	);
};
