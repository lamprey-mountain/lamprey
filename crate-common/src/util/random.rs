/// fill a buffer with cryptographically secure random bytes
pub fn fill(bytes: &mut [u8]) {
    rand::fill(bytes);
}
