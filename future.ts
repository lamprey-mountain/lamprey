// future ideas

// object store + materialized views (like ufh)
// everything viewable to author only by default, visibility/acls done as part of indexing
// TODO: how to handle deletions/gc? make every user handle deletions on their own?
// optimize for lots of tiny pieces of data? batch Things?

// immutable pieces of linked data
// needs a global total order? or don't necessarily bake it in, let indexers handle it
type Thing = {
  id: string,   // uuid v7
  type: string, // namespaced
  content: any, // depends on type
}

// depends builds a dag of indexers for joins
// remote indexers can be queried too
// special indexer lang like sql? (serialize indexers as Things?)
// can indexers cause more Things to be created? what about infinite loops?
type Indexer = {
  types: Array<string>,
  depends: Array<string>,
  queries: Record<string, any>,
  init(): void,
  index(thing: Thing): void, // or maybe require it to be derived/materialized views instead of procedural
  query(query: any): any,
}

// if this is decentralized, there needs to be a way
// to send Things to other users (or share indexes/things in some other way..?)
// can this be merged to Indexer?
type Publisher = {
  types: Array<string>,
  depends: Array<string>,
  publish(thing: Thing): Array<string>,
}
