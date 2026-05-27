// TEMP: experimental ideas for resource linking
// TODO: handle federation

use uuid::Uuid;

use crate::v1::types::MediaId;

/// a link to another piece of content on lamprey
pub struct Link<T: Linkable, Constraint: LinkDeleteEffect> {
    /// the id of the target resource
    target_id: Option<T::Id>,

    /// the type of the target resource
    target_type: ResourceType,

    // NOTE: maybe add?
    // target_data: Option<T>,
    /// path to field in source where the target resource is used
    path: Vec<String>,

    _constraint: PhantomData<Constraint>,
}

pub struct LinkReverse<T: Linkable> {
    /// path to field in source where this resource is used
    path: Vec<String>,

    source_id: T::Id,
    source_type: ResourceType,
}

mod flex {
    pub trait Seal {}
}

/// prevent deletion
pub enum Required {}

/// require acknowledging a warning, nullify when acked
pub enum Warn {}

/// set this field to null
pub enum Nullify {}

/// what to do to this resource when the linked resource is deleted
pub trait LinkDeleteEffect: flex::Seal {}

impl flex::Seal for Required {}
impl flex::Seal for Nullify {}

impl LinkDeleteEffect for Required {}
impl LinkDeleteEffect for Nullify {}

/// the type of a resource
pub enum ResourceType {
    Media,
    User,
    Channel,
    Message,
    MessageVersion,
    // how does Embed work?
}

// pub enum LinkType {
//     Media,
//     // Message,
//     // MessageVersion,
//     // UserAvatar,
//     // UserBanner,
//     // ChannelIcon,
//     // RoomIcon,
//     // RoomBanner,
//     // Embed,
//     // CustomEmoji,
//     // Script,
//     // ScriptVersion,
// }

/// this resource makes use of the links system
///
/// is can be linked to and has links to other resources
// TODO: macro?
pub trait Links {
    type Id;

    /// get the id of this resource
    fn id(&self) -> Self::Id;

    /// get the type of this resource
    fn resource_type(&self) -> ResourceType;

    /// visit the links this resource has
    fn visit_links<V: LinksVisitor>(&self, visitor: &mut V);
}

/// trait for collecting links for a resource
pub trait LinksVisitor {
    fn visit(&mut self, link: Link);
}

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

// utility for collecting all links
impl LinksVisitor for Vec<Link> {
    fn visit(&mut self, link: Link) {
        self.push(link);
    }
}

// ------ 8< snip ------
// below will be deleted

// // trait for data
// pub trait DataLinks {
//     /// update the links for a resource
//     fn links_update(&mut self, target: Uuid, diff: LinkDiff) -> Result<()>;

//     /// retrieve the reverse links for a resource
//     fn links_reverse(&mut self, target: Uuid) -> Result<Vec<LinkReverse>>;

//     // see below
//     fn links_remove_reverse(&mut self, target: Uuid) -> Result<()>;
// }

/// example usage
mod next {
    use std::marker::PhantomData;

    use crate::{
        v1::types::{MediaId, UserId},
        v2::types::{
            links::{Link, LinkDiff, LinkReverse, Links, Nullify, Required, ResourceType},
            media::Media,
        },
    };

    // #[derive(Links)]
    // #[links] // alternative? if i need to modify stuff
    struct User {
        avatar: Link<Media, Required>,
        banner: Link<Media, Required>,

        // expands to this...
        avatar_id: MediaId,
        banner_id: MediaId,
    }

    // ...and impls this
    impl Links for User {
        type Id = UserId;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn resource_type(&self) -> super::ResourceType {
            ResourceType::User
        }

        fn visit_links<V: super::LinksVisitor>(&self, visitor: &mut V) {
            visitor.visit(Link {
                target_id: self.avatar_id,
                target_type: ResourceType::Media,
                path: vec!["avatar_id".to_owned()],
                _constraint: PhantomData,
            });
            visitor.visit(Link {
                target_id: self.banner_id,
                target_type: ResourceType::Media,
                path: vec!["banner_id".to_owned()],
                _constraint: PhantomData,
            });
        }
    }

    struct Room {
        afk_channel: Link<Channel, Nullify>,
        welcome_channel: Link<Channel, Nullify>,
    }

    // TODO: abstract this into data.links_remove_reverse(*channel_id)?
    fn example_delete_channel() {
        // add this to wherever channel deletion is done
        let reverse_links = data.data_links_reverse(*channel_id).await?;
        let reverse: LinkReverse<_> = reverse_links[0]; // use for loop
        if reverse.constraint() == required {
            return Err;
        } else {
            data.data_links_update(
                reverse.source_id,
                LinkDiff {
                    removed: vec![Link {
                        target_id: *channel_id,
                        target_type: reverse.source_type,
                        path: reverse.path,
                        _constraint: PhantomData,
                    }],
                    kept: vec![],
                    added: vec![],
                },
            )
            .await?;
            // TODO: somehow clear room table afk_channel column from the database too..?
        }
    }

    // TODO: how does it work with messages?
}

// add these routes for debugging
mod http {
    // GET /debug/resource/{uuid}/links
    // GET /debug/resource/{uuid}/revlinks

    struct Links {
        links: Vec<Link>,
    }

    struct Revlinks {
        links: Vec<LinkReverse>,
    }
}
