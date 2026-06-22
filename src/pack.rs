use flate2::Compression;
use flate2::write::ZlibEncoder;
use flate2::bufread::ZlibDecoder;
use std::io::{Read, Write};
use std::fs::File;
use std::collections::HashSet;
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

fn compress_payload(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

struct DiscoveredObject {
    obj_type: u8,
    payload: Vec<u8>,
}

fn read_loose_object(sha1: &[u8; 20]) -> std::io::Result<DiscoveredObject> {
    let hash_hex = hex::encode(sha1);
    let (dir_part, file_part) = hash_hex.split_at(2);
    let object_path = format!(".git/objects/{}/{}", dir_part, file_part);

    let file = File::open(object_path)?;
    let mut decoder = ZlibDecoder::new(std::io::BufReader::new(file));
    let mut decoded_bytes = Vec::new();
    decoder.read_to_end(&mut decoded_bytes)?;

    let null_pos = decoded_bytes.iter().position(|&b| b == 0).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Malformed loose object header")
    })?;

    let header_str = std::str::from_utf8(&decoded_bytes[..null_pos]).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Header is not valid UTF-8")
    })?;

    let type_str = header_str.split_whitespace().next().unwrap_or("");
    let obj_type = match type_str {
        "commit" => 1,
        "tree" => 2,
        "blob" => 3,
        _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unknown git object type")),
    };

    let payload = decoded_bytes[null_pos + 1..].to_vec();

    Ok(DiscoveredObject { obj_type, payload })
}

fn parse_tree_shas(tree_payload: &[u8]) -> Vec<[u8; 20]> {
    let mut shas = Vec::new();
    let mut cursor = 0;

    while cursor < tree_payload.len() {
        if let Some(null_offset) = tree_payload[cursor..].iter().position(|&b| b == 0) {
            let sha_start = cursor + null_offset + 1;
            let sha_end = sha_start + 20;

            if sha_end <= tree_payload.len() {
                let mut sha1 = [0u8; 20];
                sha1.copy_from_slice(&tree_payload[sha_start..sha_end]);
                shas.push(sha1);
            }
            cursor = sha_end;
        } else {
            break;
        }
    }
    shas
}

fn collect_tree_objects(
    tree_sha1: &[u8; 20],
    objects: &mut Vec<DiscoveredObject>,
    visited: &mut HashSet<[u8; 20]>,
) -> std::io::Result<()> {
    if !visited.insert(*tree_sha1) {
        return Ok(()); 
    }

    let tree_obj = read_loose_object(tree_sha1)?;
    let child_shas = parse_tree_shas(&tree_obj.payload);
    
    objects.push(tree_obj);

    for child_sha in child_shas {
        let child_obj = read_loose_object(&child_sha)?;
        match child_obj.obj_type {
            3 => {
                if visited.insert(child_sha) {
                    objects.push(child_obj);
                }
            }
            2 => {
                collect_tree_objects(&child_sha, objects, visited)?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn create_dynamic_packfile(commit_sha1: &[u8; 20]) -> std::io::Result<(Vec<u8>, String)> {
    let mut discovered_objects = Vec::new();
    let mut visited = HashSet::new();

    let commit_obj = read_loose_object(commit_sha1)?;
    let commit_text = std::str::from_utf8(&commit_obj.payload)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Commit body text error"))?;
    
    let tree_hex = commit_text.lines().next()
        .and_then(|line| line.strip_prefix("tree "))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "No tree reference in commit"))?;

    let mut root_tree_sha1 = [0u8; 20];
    hex::decode_to_slice(tree_hex, &mut root_tree_sha1)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid tree hex in commit"))?;

    visited.insert(*commit_sha1);
    discovered_objects.push(commit_obj);

    collect_tree_objects(&root_tree_sha1, &mut discovered_objects, &mut visited)?;

    let mut pack = Vec::new();

    pack.extend_from_slice(b"PACK");
    pack.extend_from_slice(&[0, 0, 0, 2]);
    pack.extend_from_slice(&(discovered_objects.len() as u32).to_be_bytes());

    for obj in discovered_objects {
        pack.extend_from_slice(&encode_object_header(obj.obj_type, obj.payload.len() as u64));
        pack.extend_from_slice(&compress_payload(&obj.payload));
    }

    let mut hasher = Sha1::new();
    hasher.update(&pack);
    let checksum = hasher.finalize();
    pack.extend_from_slice(&checksum);

    Ok((pack, hex::encode(commit_sha1)))
}
