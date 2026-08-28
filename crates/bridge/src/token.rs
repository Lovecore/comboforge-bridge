/// Pairing token: 100 bits from the OS RNG, rendered as Crockford base32 in
/// groups of four (no ambiguous characters; survives being read aloud on a
/// Discord call).
///
/// Stored PLAINTEXT in config.json, deliberately: DPAPI-style encryption
/// protects against another *user*, and another user cannot read your
/// %LOCALAPPDATA% anyway; against a same-user process it is decoration that
/// would add unsafe FFI and an audit question. A same-user process could
/// also simply call XInput itself -- this program grants a local attacker
/// nothing they did not already have (docs/THREAT-MODEL.md).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn generate() -> String {
    let mut raw = [0u8; 13]; // 104 bits; we consume 100
    getrandom::getrandom(&mut raw).expect("OS RNG");
    let mut out = String::with_capacity(24);
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut emitted = 0;
    for byte in raw {
        acc = (acc << 8) | byte as u32;
        bits += 8;
        while bits >= 5 && emitted < 20 {
            bits -= 5;
            let idx = ((acc >> bits) & 31) as usize;
            out.push(CROCKFORD[idx] as char);
            emitted += 1;
            if emitted % 4 == 0 && emitted < 20 {
                out.push('-');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_and_uniqueness() {
        let a = generate();
        let b = generate();
        assert_eq!(a.len(), 24); // 20 chars + 4 dashes
        assert!(a.split('-').all(|g| g.len() == 4));
        assert!(a
            .chars()
            .all(|c| c == '-' || CROCKFORD.contains(&(c as u8))));
        assert_ne!(a, b);
    }
}
