// TODO: text/content processing (eg. mentions)

pub mod convert {
    use crate::{bridge_old::ReactionKey, prelude::*};

    impl From<lamprey::ReactionKey> for ReactionKey {
        fn from(key: lamprey::ReactionKey) -> Self {
            Self::Lamprey(key)
        }
    }

    impl From<discord::ReactionType> for ReactionKey {
        fn from(key: discord::ReactionType) -> Self {
            Self::Discord(key)
        }
    }

    #[cfg(any())]
    pub mod embed {
        // how do i handle media downloads

        impl From<discord::Embed> for lamprey::EmbedCreate {
            fn from(value: discord::Embed) -> Self {
                todo!()
            }
        }

        impl From<lamprey::Embed> for discord::CreateEmbed {
            fn from(value: lamprey::Embed) -> Self {
                let mut e = discord::CreateEmbed::new();

                if let Some(u) = value.url {
                    e = e.url(u)
                }

                // TODO

                e
            }
        }
    }
}

pub mod mentions;
