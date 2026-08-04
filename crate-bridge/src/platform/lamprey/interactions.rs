// TODO

// on discord /link, send a new message
// - with buttons to accept/reject link
// - expire and delete after 5 minutes

use common::v1::types::{
    MessageCreate,
    components::{ComponentCreate, ComponentType, Components},
};

fn a() -> MessageCreate {
    MessageCreate {
        content: todo!(),
        attachments: todo!(),
        reply_id: todo!(),
        embeds: todo!(),
        mentions: todo!(),
        metadata: todo!(),
        components: Some(Components {
            inner: vec![ComponentCreate {
                id: None,
                ty: ComponentType::Container {
                    components: vec![todo!()],
                    color: todo!(),
                },
                allow: todo!(),
            }],
        }),
        ephemeral: todo!(),
    }
}
