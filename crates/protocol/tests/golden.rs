//! The cross-repo drift tripwire.
//!
//! Every line of /protocol/fixtures/v1.jsonl must deserialize and
//! re-serialize to the EXACT same bytes -- and the ComboForge monorepo runs
//! the same file through the page's parser. If either side drifts, a test
//! fails before a streamer ever sees a garbled message.
//!
//! One deliberate exception: the line whose type starts with "__future"
//! exists for the OTHER side -- it proves the page ignores unknown message
//! types. This implementation is allowed to refuse it.

use comboforge_bridge_protocol::ServerMessage;

const FIXTURES: &str = include_str!("../../../protocol/fixtures/v1.jsonl");

#[test]
fn every_fixture_round_trips_byte_exact() {
    let mut checked = 0;
    let mut future_lines = 0;
    for line in FIXTURES.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("\"__future") {
            future_lines += 1;
            continue;
        }
        let parsed: ServerMessage = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("fixture did not parse: {e}\n  {line}"));
        let reserialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(reserialized, line, "serialization drifted from the fixture");
        checked += 1;
    }
    assert!(
        checked >= 8,
        "fixture file looks truncated ({checked} lines)"
    );
    assert_eq!(future_lines, 1, "exactly one future-type line belongs here");
}

#[test]
fn unknown_client_message_types_are_ignored_not_fatal() {
    use comboforge_bridge_protocol::parse_client_message;
    assert!(parse_client_message("{\"type\":\"detach\",\"whatever\":1}").is_none());
    assert!(parse_client_message("not json at all").is_none());
    let auth = parse_client_message(
        "{\"type\":\"auth\",\"token\":\"KX7M-9QT2\",\"protocol\":{\"major\":1,\"minorMin\":0,\"minorMax\":0},\"client\":\"comboforge-web/2026.08\"}",
    );
    assert!(auth.is_some());
}
