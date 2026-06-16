// TEMP: experimental ideas for resource linking
// TODO: handle federation

#![allow(unused)] // TEMP

use uuid::Uuid;

/// a link to another piece of content on lamprey
pub struct Link {
    /// the id of the target resource
    target_id: Option<Uuid>,

    /// the type of the target resource
    target_type: ResourceType,

    /// path to field in source where the target resource is used
    path: Vec<String>,

    constraint: Constraint,
    // reversed: bool,
}

// // populated link
// pub struct Link2<'a, T> {
//     data: &mut 'a T,
// }

// pub trait Links2 {
//     fn visit() -> ();
//     fn visit_links(&self) -> ();
//     fn visit_links_mut(&mut self) -> ();
// }

/// resolving media refs
mod media {
    use std::collections::HashMap;

    use crate::{
        v1::types::{MessageAttachmentCreateType, MessageCreate},
        v2::types::{MediaId, media::MediaReference},
    };

    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    use url::Url;
    #[cfg(feature = "utoipa")]
    use utoipa::ToSchema;

    /// A reference to a piece of media to be used.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    // #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    pub struct MediaRef {
        ty: MediaRefType,
        resolved: Option<MediaId>,
    }

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

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(untagged))]
    pub enum MediaRefType {
        /// Use this piece of uploaded media. Prefer using this whenever possible.
        Media { media_id: MediaId },

        /// Shortcut to download media from a url. Saves a few requests for uploading.
        Url { source_url: Url },

        /// Shortcut to create media from form data. Only usable if the request body is multipart/form-data.
        Attachment { media_index: u64 },
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

    pub trait MediaResolvable {
        fn resolve_media(&mut self, resolver: &dyn Fn(&mut MediaRef));
    }

    // impl MediaResolvable for MessageCreate {
    //     fn resolve_media(&mut self, resolver: &dyn Fn(&mut MediaRef)) {
    //         for att in &self.attachments {
    //             match &mut att.ty {
    //                 MessageAttachmentCreateType::Media { media, .. } => resolver(media),
    //             }
    //         }
    //     }
    // }
}

pub enum Constraint {
    /// prevent deletion
    Required,

    /// require acknowledging a warning, nullify when acked
    Warn,

    /// set this field to null
    ///
    /// only valid for `Option<T>` fields
    Nullify,
}

/// this resource makes use of the links system
///
/// is can be linked to and has links to other resources
// TODO: derive macro
pub trait Links {
    // type Id;

    // /// get the id of this resource
    // fn id(&self) -> Self::Id;

    // /// get the type of the target resource
    // // fn resource_type(&self) -> LinkType;
    // fn resource_type(&self) -> String;

    /// visit the links this resource has
    fn visit_links<V: LinksVisitor>(&self, visitor: &mut V);
}

/// the type of a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Media,
    User,
    // Channel,
    // Message,
    // MessageVersion,
    // how does Embed work?
}

/// trait for collecting links for a resource
pub trait LinksVisitor {
    fn visit(&mut self, link: Link);
}

// utility for collecting all links
impl LinksVisitor for Vec<Link> {
    fn visit(&mut self, link: Link) {
        self.push(link);
    }
}

pub mod resolve {
    use crate::v2::types::{MediaId, media::Media};

    pub struct Query {
        media: Vec<MediaId>,
        // users: Vec<UserId>,
        // room member, thread member, message, channels/threads, etc
        // applications, webhooks, tags?
    }

    pub struct Resolved {
        media: Vec<Media>,
    }
}

pub mod diff {
    use crate::v2::types::links::{Link, Links};

    pub struct LinkDiff {
        pub removed: Vec<Link>,
        pub kept: Vec<Link>,
        pub added: Vec<Link>,
    }

    impl LinkDiff {
        /// calculate which links were added/removed
        pub fn diff<T: Links>(old: &T, new: &T) -> Self {
            todo!()
        }

        /// create a new diff that creates all these links
        pub fn create<T: Links>(resource: &T) -> Self {
            todo!()
        }

        /// create a new diff that deletes all these links
        pub fn delete<T: Links>(resource: &T) -> Self {
            todo!()
        }
    }
}

mod example {
    use std::marker::PhantomData;

    use crate::v2::types::{
        MediaId,
        links::{Constraint, Link, Links, ResourceType},
    };

    use super::LinksVisitor;

    struct User {
        avatar_id: Option<MediaId>,
        banner_id: Option<MediaId>,
    }

    // with derive macro
    // #[derive(Links)]
    // struct User {
    //     // can be Option<T> or T
    //     #[link(Media, constraint = Nullify)]
    //     avatar_id: Option<MediaId>,

    //     #[link(Media, constraint = Nullify)]
    //     banner_id: Option<MediaId>,
    // }

    impl Links for User {
        // type Id = UserId;

        // fn id(&self) -> Self::Id {
        //     self.id
        // }

        // fn resource_type(&self) -> super::ResourceType {
        //     ResourceType::User
        // }

        fn visit_links<V: LinksVisitor>(&self, visitor: &mut V) {
            visitor.visit(Link {
                target_id: self.avatar_id.map(|i| *i),
                target_type: ResourceType::Media,
                path: vec!["avatar_id".to_owned()],
                constraint: Constraint::Nullify,
            });
            visitor.visit(Link {
                target_id: self.banner_id.map(|i| *i),
                target_type: ResourceType::Media,
                path: vec!["banner_id".to_owned()],
                constraint: Constraint::Nullify,
            });
        }
    }
}

// // ------ 8< snip ------
// // below will be deleted
//
// // // trait for data
// // pub trait DataLinks {
// //     /// update the links for a resource
// //     fn links_update(&mut self, target: Uuid, diff: LinkDiff) -> Result<()>;
//
// //     /// retrieve the reverse links for a resource
// //     fn links_reverse(&mut self, target: Uuid) -> Result<Vec<Link>>;
//
// //     // see below
// //     fn links_remove_reverse(&mut self, target: Uuid) -> Result<()>;
// // }
//
// // add these routes for debugging
// mod http {
//     // GET /link/{uuid}/forward
//     // GET /link/{uuid}/reverse
//
//     struct Links {
//         links: Vec<Link>,
//     }
// }

// // TODO: abstract this into data.links_remove_reverse(*channel_id)?
// fn example_delete_channel() {
//     // add this to wherever channel deletion is done
//     let reverse_links = data.data_links_reverse(*channel_id).await?;
//     let reverse: LinkReverse<_> = reverse_links[0]; // use for loop
//     if reverse.constraint() == required {
//         return Err;
//     } else {
//         data.data_links_update(
//             reverse.source_id,
//             LinkDiff {
//                 removed: vec![Link {
//                     target_id: *channel_id,
//                     target_type: reverse.source_type,
//                     path: reverse.path,
//                     _constraint: PhantomData,
//                 }],
//                 kept: vec![],
//                 added: vec![],
//             },
//         )
//         .await?;
//         // TODO: somehow clear room table afk_channel column from the database too..?
//     }
// }
