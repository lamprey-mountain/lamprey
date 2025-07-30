import type { Thread } from "sdk";
import { For, type VoidProps } from "solid-js";
import { permissions } from "../room_settings/Roles.tsx";

export function Permissions(_props: VoidProps<{ thread: Thread }>) {
  return (
    <>
      <h2>Permissions</h2>
      <div style="display:flex">
        <div>list of roles/users, button to add role/user</div>
        <div>
          <For each={permissions}>{(p) => <li>{p.id} button: allow/default/deny</li>}</For>
        </div>
      </div>
    </>
  );
}

