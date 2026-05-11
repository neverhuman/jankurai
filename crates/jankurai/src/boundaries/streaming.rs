pub const STREAMING_CLIENT_MARKERS: &[&str] = &[
    "rdkafka",
    "kafkajs",
    "kafka-node",
    "tansu",
    "iggy",
    "fluvio",
    "nats",
    "redis::streams",
];

pub fn streaming_client_marker(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    STREAMING_CLIENT_MARKERS
        .iter()
        .copied()
        .find(|marker| lower.contains(marker))
}
