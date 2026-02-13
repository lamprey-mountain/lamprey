use common::v1::types::Permission;

/// compressed representation of permissions
pub struct PermissionBits(pub(super) u128);

impl From<Vec<Permission>> for PermissionBits {
    fn from(value: Vec<Permission>) -> Self {
        todo!()
    }
}

impl From<PermissionBits> for Vec<Permission> {
    fn from(value: PermissionBits) -> Self {
        todo!()
    }
}
