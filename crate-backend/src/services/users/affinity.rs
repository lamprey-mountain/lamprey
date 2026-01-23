// TODO: add method to calculate affinity from one person to another

#![allow(unused)] // TEMP: suppress warnings here for now

// impl ServiceUsers {
//     pub async fn affinity(&self, acting: UserId, target: Option<UserId>) -> Result<UserAffinity> {
//         todo!()
//     }
// }

use common::v1::types::{Relationship, RelationshipType};

/// user affinity from one person to another
///
/// used for calculating what one person can do to another outside of rooms
pub struct UserAffinity {
    // /// whether you are friends
    // // smaller than storing Relationship?
    // friends: bool,
    /// your relationship with the other user
    relationship: Relationship,

    /// the other user's relationship with you user
    relationship_reverse: Relationship,

    /// both users share a mutual room
    mutual_room: bool,

    /// both users share a mutual group dm
    mutual_gdm: bool,

    /// other user has friend requests enabled in any mutual room
    room_can_friend: bool,

    /// other user has dms enabled in any mutual room
    room_can_dm: bool,
    // /// other user has rich presence enabled in any mutual room
    // room_can_view_presence: bool,
}

impl UserAffinity {
    /// whether you can create a direct message channel with the other user
    pub fn can_dm(&self) -> bool {
        self.relationship.relation == Some(RelationshipType::Friend)
            || self.mutual_gdm
            || (self.mutual_room && self.room_can_dm)
    }

    /// whether you can send a friend request to the other user
    pub fn can_friend(&self) -> bool {
        self.mutual_gdm || (self.mutual_room && self.room_can_friend)
    }
}
