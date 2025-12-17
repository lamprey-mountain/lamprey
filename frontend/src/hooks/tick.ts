import { createSignal } from "solid-js";

const [tick_, setTick] = createSignal(Date.now());
setInterval(() => setTick(Date.now()), 1000 * 30);
export const tick = tick_;
