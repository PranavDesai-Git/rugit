use std::io::{Read, Write};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};

// Helper to format strings into Git's pkt-line format
fn encode_pkt_line(payload: &str) -> Vec<u8> {
    let len = payload.len() + 4;
    let hex_len = format!("{:04x}", len);
    let mut result = hex_len.into_bytes();
    result.extend_from_slice(payload.as_bytes());
    result
}



