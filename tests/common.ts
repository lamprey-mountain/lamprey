
import { assertEquals } from "@std/assert";

export const BASE_URL = Deno.env.get("BASE_URL") || "http://localhost:4000";

export type Tester = {
    (cfg: {
        url: string;
        method?: string;
        body?: any;
        status?: number;
    }): Promise<any>;
    user: any;
    token: string;
};

export function makeTester(token: string, user: any = null): Tester {
    const fn = async ({ url, method = "GET", body, status }: {
        url: string;
        method?: string;
        body?: any;
        status?: number;
    }) => {
        const fullUrl = url.startsWith("http") ? url : `${BASE_URL}/api/v1${url}`;
        const res = await fetch(fullUrl, {
            method,
            headers: {
                "Authorization": `Bearer ${token}`,
                "Content-Type": "application/json",
            },
            body: body ? JSON.stringify(body) : undefined,
        });

        if (status !== undefined) {
            assertEquals(res.status, status, `Expected status ${status} but got ${res.status} for ${method} ${url}`);
        }

        if (res.status === 204) return null;
        return await res.json();
    };
    const tester = fn as Tester;
    tester.token = token;
    tester.user = user;
    return tester;
}

export async function createTester(name: string): Promise<Tester> {
    // 1. Create session
    const sessionRes = await fetch(`${BASE_URL}/api/v1/session`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: `session-for-${name}` }),
    });
    const { session, token } = await sessionRes.json();

    // 2. Create guest user
    const guestRes = await fetch(`${BASE_URL}/api/v1/guest`, {
        method: "POST",
        headers: {
            "Authorization": `Bearer ${token}`,
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ name }),
    });
    const user = await guestRes.json();

    // 3. Promote to registered user
    await admin({
        url: "/admin/register-user",
        method: "POST",
        body: { user_id: user.id },
        status: 204,
    });
    
    return makeTester(token, user);
}

export const admin = makeTester("debug-admin-token");

export class SyncClient {
    private ws: WebSocket;
    private messages: any[] = [];
    private waiters: { predicate: (msg: any) => boolean; resolve: (msg: any) => void }[] = [];
    public ready: Promise<void>;

    constructor(token: string) {
        const wsUrl = BASE_URL.replace("http", "ws") + "/api/v1/sync?version=1";
        this.ws = new WebSocket(wsUrl);
        
        let resolveReady: () => void;
        this.ready = new Promise((resolve) => {
            resolveReady = resolve;
        });

        this.ws.onopen = () => {
            this.ws.send(JSON.stringify({
                type: "Hello",
                token: token,
            }));
        };

        this.ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            if (msg.op === "Ready") {
                resolveReady();
                return;
            }
            if (msg.op === "Sync") {
                const data = msg.data;
                const waiterIndex = this.waiters.findIndex(w => w.predicate(data));
                if (waiterIndex !== -1) {
                    const waiter = this.waiters.splice(waiterIndex, 1)[0];
                    waiter.resolve(data);
                } else {
                    this.messages.push(data);
                }
            }
        };
    }

    async waitFor(predicate: (msg: any) => boolean, timeout = 5000): Promise<any> {
        const existing = this.messages.findIndex(predicate);
        if (existing !== -1) {
            return this.messages.splice(existing, 1)[0];
        }

        return new Promise((resolve, reject) => {
            const t = setTimeout(() => reject(new Error("Timeout waiting for sync message")), timeout);
            this.waiters.push({
                predicate,
                resolve: (msg) => {
                    clearTimeout(t);
                    resolve(msg);
                }
            });
        });
    }

    send(msg: any) {
        this.ws.send(JSON.stringify(msg));
    }

    close() {
        this.ws.close();
    }
}
