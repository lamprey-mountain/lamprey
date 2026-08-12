import { openDB, IDBPDatabase, DBSchema } from "idb";
// import {} from "@lamprey/crypto";

// TODO: plan out and impl this

export interface E2EEDBSchema extends DBSchema {
  identity: {
    key: string; // session id?
    value: Uint8Array; // NOTE: can i use a raw array here?
  };
  groups: {
    key: string; // channel id
    value: {
      channelId: string;
      state: Uint8Array; // serialized state
      updatedAt: number;
    };
  };
  epochKeys: {
    key: [string, number];
    value: {
      channelId: string;
      epoch: number;
      key: Uint8Array;
    };
    indexes: {
      byChannelId: string;
    }
  };
}


export class Crypto { }

// const db = await openDB<E2EEDBSchema>("crypto", 1, {
//   upgrade(db) {
//     db.createObjectStore("identity");
//     db.createObjectStore("groups", { keyPath: "channelId" });

//     const epochStore = db.createObjectStore("epochKeys", { keyPath: ["channelId", "epoch"] });
//     epochStore.createIndex("byChannelId", "channelId");
//   },
// });
