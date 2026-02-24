use crate::v1::types::embed::{Embed as V1Embed, EmbedCreate as V1EmbedCreate, EmbedType as V1EmbedType};
use crate::v2::types::embed::{Embed as V2Embed, EmbedCreate as V2EmbedCreate, EmbedType as V2EmbedType};
use crate::v2::types::media::MediaReference;

impl From<V1EmbedType> for V2EmbedType {
    fn from(v1: V1EmbedType) -> Self {
        match v1 {
            V1EmbedType::Media => V2EmbedType::Media,
            V1EmbedType::Link => V2EmbedType::Link,
            V1EmbedType::Custom => V2EmbedType::Custom,
        }
    }
}

impl From<V1Embed> for V2Embed {
    fn from(v1: V1Embed) -> Self {
        V2Embed {
            id: v1.id,
            ty: v1.ty.into(),
            url: v1.url,
            canonical_url: v1.canonical_url,
            title: v1.title,
            description: v1.description,
            color: v1.color,
            media: v1.media.map(|m| m.into()),
            thumbnail: v1.thumbnail.map(|m| m.into()),
            author_name: v1.author_name,
            author_url: v1.author_url,
            author_avatar: v1.author_avatar.map(|m| m.into()),
            site_name: v1.site_name,
            site_avatar: v1.site_avatar.map(|m| m.into()),
        }
    }
}

impl From<V1EmbedCreate> for V2EmbedCreate {
    fn from(v1: V1EmbedCreate) -> Self {
        V2EmbedCreate {
            url: v1.url,
            title: v1.title,
            description: v1.description,
            color: v1.color,
            media: v1.media.map(|m| MediaReference::Media { media_id: m.id }),
            thumbnail: v1.thumbnail.map(|m| MediaReference::Media { media_id: m.id }),
            author_name: v1.author_name,
            author_url: v1.author_url,
            author_avatar: v1.author_avatar.map(|m| MediaReference::Media { media_id: m.id }),
        }
    }
}
