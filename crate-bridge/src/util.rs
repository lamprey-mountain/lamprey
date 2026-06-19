// TODO: text/content processing (eg. mentions)

#[cfg(any())]
pub mod convert {
    use crate::prelude::*;

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
