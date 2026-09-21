// TODO: make linking general purpose, merge into crate::v2::types::links

use crate::v1::types::MediaId;

pub use super::MediaReference as MediaRefType;

/// A reference to a piece of media to be used.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaRef {
    ty: MediaRefType,
    resolved: Option<MediaId>,
}

impl MediaRef {
    /// mark this as resolved with a particular id
    pub fn resolve(&mut self, media_id: MediaId) {
        self.resolved = Some(media_id);
    }

    /// get the resolved media id for this reference
    pub fn media_id(&self) -> Option<MediaId> {
        self.resolved
    }
}

// TODO: how can i implement this trait reasonably?
// TODO: can i avoid implementors needing to duplicate code between visit and resolve fns?
pub trait MediaResolvable {
    /// visit all referenced media
    fn visit<F: FnMut(&MediaRef)>(&self, visitor: F);

    /// visit all referenced media mutably
    fn resolve<F: FnMut(&mut MediaRef)>(&mut self, resolver: F);
}

// impl MediaResolvable for MessageCreate {
//     fn resolve(&self, resolver: &FnMut(&MediaRef)) {
//         for att in &self.attachments {
//             match &att.ty {
//                 MessageAttachmentCreateType::Media { media, .. } => resolver(media),
//             }
//         }
//     }
//
//     fn resolve(&mut self, resolver: &FnMut(&mut MediaRef)) {
//         for att in &mut self.attachments {
//             match &mut att.ty {
//                 MessageAttachmentCreateType::Media { media, .. } => resolver(media),
//             }
//         }
//     }
// }

#[cfg(feature = "serde")]
mod s {
    use serde::Deserialize;

    use super::{MediaRef, MediaRefType};

    impl<'de> Deserialize<'de> for MediaRef {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let ty = MediaRefType::deserialize(deserializer)?;
            Ok(MediaRef { ty, resolved: None })
        }
    }
}

#[cfg(feature = "utoipa")]
mod u {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{RefOr, schema::Schema},
    };

    use super::{MediaRef, MediaRefType};

    impl PartialSchema for MediaRef {
        fn schema() -> RefOr<Schema> {
            MediaRefType::schema()
        }
    }

    impl ToSchema for MediaRef {}
}
