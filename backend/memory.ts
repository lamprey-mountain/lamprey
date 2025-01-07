// TODO: in memory caches, in a separate file for hmr
// permission checks aren't too bad right now but look like they can get very expensive soon...
// or maybe there's a way to do it with postgres

export const permCacheRoom = new Map<string, Permissions>();
export const permCacheThread = new Map<string, Permissions>();

