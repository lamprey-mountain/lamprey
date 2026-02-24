use crate::v1::types::embed::{
    Embed as V1Embed, EmbedCreate as V1EmbedCreate, EmbedType as V1EmbedType,
};
use crate::v2::types::embed::{
    Embed as V2Embed, EmbedCreate as V2EmbedCreate, EmbedType as V2EmbedType,
};
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

impl From<V2EmbedType> for V1EmbedType {
    fn from(v2: V2EmbedType) -> Self {
        match v2 {
            V2EmbedType::Media => V1EmbedType::Media,
            V2EmbedType::Link => V1EmbedType::Link,
            V2EmbedType::Custom => V1EmbedType::Custom,
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

impl From<V2Embed> for V1Embed {
    fn from(v2: V2Embed) -> Self {
        V1Embed {
            id: v2.id,
            ty: v2.ty.into(),
            url: v2.url,
            canonical_url: v2.canonical_url,
            title: v2.title,
            description: v2.description,
            color: v2.color,
            media: v2.media.map(|m| m.into()),
            thumbnail: v2.thumbnail.map(|m| m.into()),
            author_name: v2.author_name,
            author_url: v2.author_url,
            author_avatar: v2.author_avatar.map(|m| m.into()),
            site_name: v2.site_name,
            site_avatar: v2.site_avatar.map(|m| m.into()),
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
            thumbnail: v1
                .thumbnail
                .map(|m| MediaReference::Media { media_id: m.id }),
            author_name: v1.author_name,
            author_url: v1.author_url,
            author_avatar: v1
                .author_avatar
                .map(|m| MediaReference::Media { media_id: m.id }),
        }
    }
}

impl From<V2EmbedCreate> for V1EmbedCreate {
    fn from(v2: V2EmbedCreate) -> Self {
        V1EmbedCreate {
            url: v2.url,
            title: v2.title,
            description: v2.description,
            color: v2.color,
            media: v2.media.and_then(|m| match m {
                MediaReference::Media { media_id } => {
                    Some(crate::v1::types::media::MediaRef { id: media_id })
                }
                _ => None,
            }),
            thumbnail: v2.thumbnail.and_then(|m| match m {
                MediaReference::Media { media_id } => {
                    Some(crate::v1::types::media::MediaRef { id: media_id })
                }
                _ => None,
            }),
            author_name: v2.author_name,
            author_url: v2.author_url,
            author_avatar: v2.author_avatar.and_then(|m| match m {
                MediaReference::Media { media_id } => {
                    Some(crate::v1::types::media::MediaRef { id: media_id })
                }
                _ => None,
            }),
        }
    }
}
