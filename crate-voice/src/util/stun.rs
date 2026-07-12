/// Parse a raw UDP packet to extract the STUN USERNAME attribute (if present)
///
/// This is used to map an unknown source address to a peer during the ICE handshake.
// TODO: check if this is correct
pub fn extract_local_ufrag(data: &[u8]) -> Option<String> {
    if data.len() < 20 {
        return None;
    }

    // STUN magic cookie must be 0x2112A442
    if data[4..8] != [0x21, 0x12, 0xA4, 0x42] {
        return None;
    }

    let mut offset = 20; // Skip 20-byte STUN header
    while offset + 4 <= data.len() {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > data.len() {
            return None;
        }

        if attr_type == 0x0006 {
            // USERNAME attribute
            let username_bytes = &data[offset..offset + attr_len];
            if let Ok(username_str) = std::str::from_utf8(username_bytes) {
                // The STUN username is formatted as "local_ufrag:remote_ufrag"
                // We extract and return the local_ufrag (our server's ufrag)
                return username_str.split(':').next().map(|s| s.to_string());
            }
        }

        // STUN attributes are padded to 4-byte boundaries
        let padded_len = (attr_len + 3) & !3;
        offset += padded_len;
    }

    None
}

#[cfg(test)]
mod tests {
    // TODO: add tests
}
