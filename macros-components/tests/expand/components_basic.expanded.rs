use lamprey_macros::{user_id, room_id};
const ADMIN: UserId = Id {
    inner: ::uuid::Uuid::from_bytes([
        85u8, 14u8, 132u8, 0u8, 226u8, 155u8, 65u8, 212u8, 167u8, 22u8, 68u8, 102u8,
        85u8, 68u8, 0u8, 0u8,
    ]),
    phantom: ::std::marker::PhantomData::<UserId>,
};
const HOME: RoomId = Id {
    inner: ::uuid::Uuid::from_bytes([
        107u8, 167u8, 184u8, 16u8, 157u8, 173u8, 17u8, 209u8, 128u8, 180u8, 0u8, 192u8,
        79u8, 212u8, 48u8, 200u8,
    ]),
    phantom: ::std::marker::PhantomData::<RoomId>,
};
