// for things ike buttons
// somewhat copied from discord since they do things reasonably

// TODO: create new type
type InteractionId = ();

/// an interaction was created
struct Interaction {
    id: InteractionId,
}

// POST /interaction/{interaction_id}/callback
struct InteractionResponse {}
