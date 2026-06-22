use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;
use sha1::{Sha1, Digest};

pub fn encode_object_header(object_type: u8, mut size: u64) -> Vec<u8> {
    let mut header = Vec::new();
    let size_lower_4_bits = (size & 0x0F) as u8;
    let type_shifted = object_type << 4;
    let mut first_byte = type_shifted | size_lower_4_bits;
    size >>= 4;
    if size > 0 { first_byte |= 0x80; }
    header.push(first_byte);

    while size > 0 {
        let mut byte = (size & 0x7F) as u8;
        size >>= 7;
        if size > 0 { byte |= 0x80; }
        header.push(byte);
    }
    header
}

/// Helper to compress an object payload using zlib
fn compress_payload(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Computes the standard Git loose object SHA-1 hash
fn compute_git_sha(obj_type: &str, body: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(format!("{} {}\0", obj_type, body.len()).as_bytes());
    hasher.update(body);
    let result = hasher.finalize();
    let mut array = [0u8; 20];
    array.copy_from_slice(&result);
    array
}

/// Generates a complete, functional packfile containing a Blob, Tree, and Commit.
/// Returns a tuple of (packfile_bytes, commit_hex_sha)
pub fn create_final_packfile() -> (Vec<u8>, String) {
    let mut pack = Vec::new();

    // 1. Packfile Header: 'PACK', Version 2, Object Count 3
    pack.extend_from_slice(b"PACK");
    pack.extend_from_slice(&[0, 0, 0, 2]); 
    pack.extend_from_slice(&[0, 0, 0, 3]); // 3 objects inside!

    // ==========================================
    // OBJECT 1: The Blob ("hello world")
    // ==========================================
    let blob_body = b"hello world";
    let blob_sha = compute_git_sha("blob", blob_body);
    
    pack.extend_from_slice(&encode_object_header(3, blob_body.len() as u64));
    pack.extend_from_slice(&compress_payload(blob_body));

    // ==========================================
    // OBJECT 2: The Tree (maps "hello.txt" -> Blob)
    // Format: "[mode] [filename]\0[20-byte binary SHA]"
    // ==========================================
    let mut tree_body = Vec::new();
    tree_body.extend_from_slice(b"100644 hello.txt\0");
    tree_body.extend_from_slice(&blob_sha);
    let tree_sha = compute_git_sha("tree", &tree_body);

    pack.extend_from_slice(&encode_object_header(2, tree_body.len() as u64));
    pack.extend_from_slice(&compress_payload(&tree_body));

    // ==========================================
    // OBJECT 3: The Commit (points to our Tree)
    // ==========================================
    let tree_hex = hex::encode(tree_sha);
    let commit_body = format!(
        "tree {}\n\
        author Engineer <engineer@example.com> 1718910693 +0000\n\
        committer Engineer <engineer@example.com> 1718910693 +0000\n\
        \n\
        Our first native custom push!\n",
        tree_hex
    ).into_bytes();
    let commit_sha = compute_git_sha("commit", &commit_body);
    let commit_hex = hex::encode(commit_sha);

    pack.extend_from_slice(&encode_object_header(1, commit_body.len() as u64));
    pack.extend_from_slice(&compress_payload(&commit_body));

    // ==========================================
    // TRAILER: Dynamic SHA-1 Checksum
    // ==========================================
    let mut hasher = Sha1::new();
    hasher.update(&pack);
    let checksum = hasher.finalize();
    pack.extend_from_slice(&checksum);

    (pack, commit_hex)
}