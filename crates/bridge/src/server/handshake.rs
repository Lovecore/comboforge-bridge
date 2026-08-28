//! The security boundary, as pure functions: the origin allowlist checked at
//! the HTTP upgrade (a rejected page gets 403 and never upgrades), and the
//! pairing-token check on the first message. Pure so the acceptance matrix
//! is a table of unit tests rather than a claim.

use comboforge_bridge_protocol::{ClientMessage, PROTOCOL_MAJOR};

/// Compiled-in allowlist. The config file can ADD origins (printed loudly at
/// startup so a quiet edit cannot widen access invisibly); it can never
/// remove these.
pub const BUILTIN_ORIGINS: [&str; 4] = [
    "https://comboforge.gg",
    "https://www.comboforge.gg",
    "http://localhost:5173",
    "http://127.0.0.1:5173",
];

/// Hand-typed config entries carry hand-typed mistakes -- a trailing slash,
/// an uppercase host, stray whitespace -- and an Origin header never has any
/// of those. Normalizing BOTH sides forgives the paste without widening the
/// gate: `https://evil.example` does not become allowed by lowercasing it.
pub fn normalize_origin(origin: &str) -> String {
    origin.trim().trim_end_matches('/').to_ascii_lowercase()
}

/// A browser always sends Origin and page JS cannot forge it. A native
/// client can forge anything -- which is fine and the threat model says so:
/// a native process on your machine could simply read XInput itself, so
/// this gate is aimed at WEB PAGES, the only attacker it can and must stop.
pub fn origin_allowed(origin: Option<&str>, extra: &[String]) -> bool {
    let Some(origin) = origin else { return false };
    let origin = normalize_origin(origin);
    BUILTIN_ORIGINS.iter().any(|o| *o == origin)
        || extra.iter().any(|o| normalize_origin(o) == origin)
}

#[derive(Debug, PartialEq)]
pub enum AuthOutcome {
    /// Token matched, major compatible: serve this client.
    Accept,
    /// Wrong token: close 4401 after an in-band error message.
    BadToken,
    /// Incompatible protocol major: close 4426 so the page can say
    /// "update the bridge" vs "refresh the page" instead of guessing.
    ProtocolMismatch { server_major: u32 },
    /// Not an auth message at all (unknown type, garbage): the client gets
    /// one chance; anything else pre-auth closes 4400.
    NotAuth,
}

pub fn check_auth(raw: &str, expected_token: &str) -> AuthOutcome {
    match comboforge_bridge_protocol::parse_client_message(raw) {
        Some(ClientMessage::Auth {
            token, protocol, ..
        }) => {
            if protocol.major != PROTOCOL_MAJOR {
                AuthOutcome::ProtocolMismatch {
                    server_major: PROTOCOL_MAJOR,
                }
            } else if constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
                AuthOutcome::Accept
            } else {
                AuthOutcome::BadToken
            }
        }
        None => AuthOutcome::NotAuth,
    }
}

/// Not strictly necessary on loopback (no timing attacker worth the name),
/// but it costs four lines and removes a question an auditor would
/// otherwise have to think about.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTH_OK: &str = r#"{"type":"auth","token":"GOOD","protocol":{"major":1,"minorMin":0,"minorMax":0},"client":"test"}"#;

    #[test]
    fn hand_typed_extra_origins_are_forgiven_their_formatting() {
        // The exact traps from the first real field test: trailing slash,
        // stray whitespace, and case -- none of which an Origin header has.
        let extra = vec![
            "https://comboforge-git-branch-team.vercel.app/".to_string(),
            "  https://Staging.Example  ".to_string(),
        ];
        assert!(origin_allowed(
            Some("https://comboforge-git-branch-team.vercel.app"),
            &extra
        ));
        assert!(origin_allowed(Some("https://staging.example"), &extra));
        assert!(!origin_allowed(
            Some("https://comboforge-git-OTHER-team.vercel.app"),
            &extra
        ));
    }

    #[test]
    fn origin_acceptance_matrix() {
        let extra = vec!["https://staging.example".to_string()];
        assert!(origin_allowed(Some("https://comboforge.gg"), &[]));
        assert!(origin_allowed(Some("http://localhost:5173"), &[]));
        assert!(origin_allowed(Some("https://staging.example"), &extra));
        assert!(!origin_allowed(Some("https://evil.example"), &extra));
        assert!(!origin_allowed(
            Some("https://comboforge.gg.evil.example"),
            &[]
        ));
        assert!(!origin_allowed(None, &extra), "missing Origin is rejected");
    }

    #[test]
    fn auth_matrix() {
        assert_eq!(check_auth(AUTH_OK, "GOOD"), AuthOutcome::Accept);
        assert_eq!(check_auth(AUTH_OK, "OTHER"), AuthOutcome::BadToken);
        assert_eq!(
            check_auth(
                r#"{"type":"auth","token":"GOOD","protocol":{"major":2,"minorMin":0,"minorMax":0},"client":"t"}"#,
                "GOOD"
            ),
            AuthOutcome::ProtocolMismatch { server_major: 1 }
        );
        assert_eq!(
            check_auth(r#"{"type":"detach"}"#, "GOOD"),
            AuthOutcome::NotAuth
        );
        assert_eq!(check_auth("garbage", "GOOD"), AuthOutcome::NotAuth);
    }
}
