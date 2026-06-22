use flate2::Compression;
use flate2::write::ZlibEncoder;
use hex::encode;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::time::UNIX_EPOCH;

pub fn init() {
    let init_folders = vec![
        ".git",
        ".git/objects",
        ".git/refs",
        ".git/refs/heads",
        ".git/refs/tags",
    ];
    let init_files = vec![
        (".git/HEAD", "ref: refs/heads/main\n"),
        (
            ".git/config",
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n",
        ),
        (
            ".git/description",
            "Unnamed repository; edit this file 'description' to name the repository.\n",
        ),
    ];
    for path in init_folders {
        match fs::create_dir_all(path) {
            Ok(_) => log::info!("{} created", path),
            Err(e) => {
                log::error!("init err @ {}:{}", path, e);
                return;
            }
        }
    }
    for (path, content) in init_files {
        match fs::write(path, content) {
            Ok(_) => log::info!("{} created", path),
            Err(e) => {
                log::error!("init err: {}", e);
                return;
            }
        }
    }
    log::info!("git initialized");
}

pub fn hash_object(filepath: &str) -> std::io::Result<[u8; 20]> {
    let file_content = fs::read(filepath)?;
    let file_size = file_content.len();

    let mut blob = Vec::new();
    write!(blob, "blob {}\0", file_size)?;
    blob.extend_from_slice(&file_content);

    let hashed_blob = Sha1::digest(&blob);

    let hashed_bytes: [u8; 20] = hashed_blob.into();
    let hash_hex = encode(hashed_blob);
    let (dir_part, file_part) = hash_hex.split_at(2);
    let target_dir = format!(".git/objects/{}", dir_part);
    let target_file = format!("{}/{}", target_dir, file_part);

    fs::create_dir_all(&target_dir)?;

    let object_file = fs::File::create(target_file)?;
    let mut encoder = ZlibEncoder::new(object_file, Compression::default());
    encoder.write_all(&blob)?;
    encoder.finish()?;

    Ok(hashed_bytes)
}

#[derive(Debug, Default)]
pub struct IndexEntry {
    pub ctime_secs: u32,
    pub ctime_nanos: u32,
    pub mtime_secs: u32,
    pub mtime_nanos: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub sha1: [u8; 20],
    pub flags: u16,
    pub filepath: String,
}

impl IndexEntry {
    pub fn from_file(filepath: &str) -> Result<Self, io::Error> {
        let metadata = fs::metadata(filepath)?;
        let mut entry = IndexEntry::default();
        if let Ok(ctime) = metadata.created() {
            if let Ok(duration) = ctime.duration_since(UNIX_EPOCH) {
                entry.ctime_secs = duration.as_secs() as u32;
                entry.ctime_nanos = duration.subsec_nanos();
            }
        }
        if let Ok(mtime) = metadata.modified() {
            if let Ok(duration) = mtime.duration_since(UNIX_EPOCH) {
                entry.mtime_secs = duration.as_secs() as u32;
                entry.mtime_nanos = duration.subsec_nanos();
            }
        }

        entry.file_size = std::cmp::min(metadata.len(), u32::MAX as u64) as u32;

        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            entry.dev = metadata.dev() as u32;
            entry.ino = metadata.ino() as u32;
            entry.uid = metadata.uid();
            entry.gid = metadata.gid();

            let unix_mode = metadata.permissions().mode();
            entry.mode = if unix_mode & 0o111 != 0 {
                0o100755
            } else {
                0o100644
            };
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Some(sn) = metadata.volume_serial_number() {
                entry.dev = sn;
            }
            if let Some(fi) = metadata.file_index() {
                entry.ino = fi as u32;
            }
            entry.uid = 0;
            entry.gid = 0;

            entry.mode = if metadata.permissions().readonly() {
                0o100755
            } else {
                0o100644
            };
        }
        entry.sha1 = hash_object(filepath)?;
        let path_len = std::cmp::min(filepath.len(), 0xFFF) as u16;
        entry.flags = path_len;
        entry.filepath = filepath.to_string();

        Ok(entry)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.ctime_secs.to_be_bytes());
        bytes.extend_from_slice(&self.ctime_nanos.to_be_bytes());
        bytes.extend_from_slice(&self.mtime_secs.to_be_bytes());
        bytes.extend_from_slice(&self.mtime_nanos.to_be_bytes());
        bytes.extend_from_slice(&self.dev.to_be_bytes());
        bytes.extend_from_slice(&self.ino.to_be_bytes());
        bytes.extend_from_slice(&self.mode.to_be_bytes());
        bytes.extend_from_slice(&self.uid.to_be_bytes());
        bytes.extend_from_slice(&self.gid.to_be_bytes());
        bytes.extend_from_slice(&self.file_size.to_be_bytes());
        bytes.extend_from_slice(&self.sha1);
        bytes.extend_from_slice(&self.flags.to_be_bytes());
        bytes.extend_from_slice(self.filepath.as_bytes());

        // Git alignment padding rule: 1-8 null bytes to pad the total
        // entry size (62 bytes fixed fields + path length) to a multiple of 8.
        let entry_len_without_padding = 62 + self.filepath.len();
        let mut padding = 1;
        while (entry_len_without_padding + padding) % 8 != 0 {
            padding += 1;
        }
        bytes.extend(std::iter::repeat(0).take(padding));

        bytes
    }
}

pub fn read_index() -> io::Result<Vec<IndexEntry>> {
    let index_path = ".git/index";
    if !std::path::Path::new(index_path).exists() {
        return Ok(Vec::new());
    }

    let mut file = fs::File::open(index_path)?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;

    if &header[0..4] != b"DIRC" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid index signature",
        ));
    }

    let _version = u32::from_be_bytes(header[4..8].try_into().unwrap());
    let entry_count = u32::from_be_bytes(header[8..12].try_into().unwrap());

    let mut entries = Vec::new();

    for _ in 0..entry_count {
        let mut fixed_fields = [0u8; 62];
        file.read_exact(&mut fixed_fields)?;

        let ctime_secs = u32::from_be_bytes(fixed_fields[0..4].try_into().unwrap());
        let ctime_nanos = u32::from_be_bytes(fixed_fields[4..8].try_into().unwrap());
        let mtime_secs = u32::from_be_bytes(fixed_fields[8..12].try_into().unwrap());
        let mtime_nanos = u32::from_be_bytes(fixed_fields[12..16].try_into().unwrap());
        let dev = u32::from_be_bytes(fixed_fields[16..20].try_into().unwrap());
        let ino = u32::from_be_bytes(fixed_fields[20..24].try_into().unwrap());
        let mode = u32::from_be_bytes(fixed_fields[24..28].try_into().unwrap());
        let uid = u32::from_be_bytes(fixed_fields[28..32].try_into().unwrap());
        let gid = u32::from_be_bytes(fixed_fields[32..36].try_into().unwrap());
        let file_size = u32::from_be_bytes(fixed_fields[36..40].try_into().unwrap());

        let mut sha1 = [0u8; 20];
        sha1.copy_from_slice(&fixed_fields[40..60]);

        let flags = u16::from_be_bytes(fixed_fields[60..62].try_into().unwrap());
        let path_len = (flags & 0x0FFF) as usize;

        let mut path_bytes = vec![0u8; path_len];
        file.read_exact(&mut path_bytes)?;
        let filepath = String::from_utf8_lossy(&path_bytes).into_owned();

        let entry_len_without_padding = 62 + path_len;
        let mut padding_len = 1;
        while (entry_len_without_padding + padding_len) % 8 != 0 {
            padding_len += 1;
        }
        let mut padding_buf = vec![0u8; padding_len];
        file.read_exact(&mut padding_buf)?;

        entries.push(IndexEntry {
            ctime_secs,
            ctime_nanos,
            mtime_secs,
            mtime_nanos,
            dev,
            ino,
            mode,
            uid,
            gid,
            file_size,
            sha1,
            flags,
            filepath,
        });
    }

    Ok(entries)
}

pub fn write_index(entries: &[IndexEntry]) -> io::Result<()> {
    let mut index_content = Vec::new();

    index_content.extend_from_slice(b"DIRC");
    index_content.extend_from_slice(&2u32.to_be_bytes());
    index_content.extend_from_slice(&(entries.len() as u32).to_be_bytes());

    for entry in entries {
        index_content.extend(entry.serialize());
    }

    let checksum = Sha1::digest(&index_content);
    index_content.extend_from_slice(&checksum);

    fs::write(".git/index", index_content)?;
    Ok(())
}

pub fn stage_file(filepath: &str) -> io::Result<()> {
    let new_entry = IndexEntry::from_file(filepath)?;
    let mut entries = read_index()?;

    entries.retain(|e| e.filepath != filepath);
    entries.push(new_entry);
    entries.sort_by(|a, b| a.filepath.cmp(&b.filepath));

    write_index(&entries)?;

    log::info!("Staged {}", filepath);
    Ok(())
}

enum TreeNode {
    Blob([u8; 20], u32),
    Tree(BTreeMap<String, TreeNode>),
}

pub fn write_tree() -> io::Result<[u8; 20]> {
    let entries = read_index()?;
    let mut root = BTreeMap::new();

    for entry in entries {
        let parts: Vec<&str> = entry.filepath.split('/').collect();
        insert_into_tree(&mut root, &parts, entry.sha1, entry.mode);
    }

    write_tree_node(&TreeNode::Tree(root))
}

fn insert_into_tree(
    current: &mut BTreeMap<String, TreeNode>,
    parts: &[&str],
    sha1: [u8; 20],
    mode: u32,
) {
    if parts.is_empty() {
        return;
    }
    let name = parts[0].to_string();
    if parts.len() == 1 {
        current.insert(name, TreeNode::Blob(sha1, mode));
    } else {
        let next_node = current
            .entry(name)
            .or_insert_with(|| TreeNode::Tree(BTreeMap::new()));

        if let TreeNode::Tree(next_tree) = next_node {
            insert_into_tree(next_tree, &parts[1..], sha1, mode);
        }
    }
}

fn write_tree_node(node: &TreeNode) -> io::Result<[u8; 20]> {
    match node {
        TreeNode::Blob(sha1, _) => Ok(*sha1),
        TreeNode::Tree(children) => {
            let mut tree_body = Vec::new();

            for (name, child) in children {
                let child_sha1 = write_tree_node(child)?;
                let mode_str = match child {
                    TreeNode::Blob(_, mode) => format!("{:o}", mode),
                    TreeNode::Tree(_) => "40000".to_string(),
                };

                write!(tree_body, "{} {}", mode_str, name)?;
                tree_body.push(0);
                tree_body.extend_from_slice(&child_sha1);
            }

            let mut tree_object = Vec::new();
            write!(tree_object, "tree {}\0", tree_body.len())?;
            tree_object.extend_from_slice(&tree_body);

            let hashed_bytes = hash_and_store_object(&tree_object)?;
            Ok(hashed_bytes)
        }
    }
}

fn hash_and_store_object(object_content: &[u8]) -> io::Result<[u8; 20]> {
    let hashed_blob = Sha1::digest(object_content);
    let hashed_bytes: [u8; 20] = hashed_blob.into();
    let hash_hex = encode(hashed_blob);

    let (dir_part, file_part) = hash_hex.split_at(2);
    let target_dir = format!(".git/objects/{}", dir_part);
    let target_file = format!("{}/{}", target_dir, file_part);

    fs::create_dir_all(&target_dir)?;
    let object_file = fs::File::create(target_file)?;
    let mut encoder = ZlibEncoder::new(object_file, Compression::default());
    encoder.write_all(object_content)?;
    encoder.finish()?;

    Ok(hashed_bytes)
}

pub fn commit(message: &str, author: &str, email: &str) -> io::Result<[u8; 20]> {
    let tree_sha1 = write_tree()?;
    let tree_hex = encode(tree_sha1);

    let mut parent_hex = String::new();
    if let Ok(head_content) = fs::read_to_string(".git/HEAD") {
        let ref_path = head_content.trim().trim_start_matches("ref: ").to_string();
        let full_ref_path = format!(".git/{}", ref_path);
        if let Ok(parent_sha) = fs::read_to_string(full_ref_path) {
            parent_hex = parent_sha.trim().to_string();
        }
    }

    let mut commit_body = Vec::new();
    writeln!(commit_body, "tree {}", tree_hex)?;
    if !parent_hex.is_empty() {
        writeln!(commit_body, "parent {}", parent_hex)?;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    writeln!(
        commit_body,
        "author {} <{}> {} +0000",
        author, email, timestamp
    )?;
    writeln!(
        commit_body,
        "committer {} <{}> {} +0000",
        author, email, timestamp
    )?;
    writeln!(commit_body)?;
    writeln!(commit_body, "{}", message)?;

    let mut commit_object = Vec::new();
    write!(commit_object, "commit {}\0", commit_body.len())?;
    commit_object.extend_from_slice(&commit_body);

    let commit_sha1 = hash_and_store_object(&commit_object)?;
    let commit_hex = encode(commit_sha1);

    if let Ok(head_content) = fs::read_to_string(".git/HEAD") {
        let ref_path = head_content.trim().trim_start_matches("ref: ").to_string();
        let full_ref_path = format!(".git/{}", ref_path);
        fs::write(full_ref_path, format!("{}\n", commit_hex))?;
    }

    log::info!("Committed successfully: {}", commit_hex);
    Ok(commit_sha1)
}

pub fn add_remote(name: &str, url: &str) -> io::Result<()> {
    let config_path = ".git/config";
    let mut config_content = fs::read_to_string(config_path)?;

    let remote_section = format!("[remote \"{}\"]", name);
    if config_content.contains(&remote_section) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Remote '{}' already exists!", name),
        ));
    }

    config_content.push_str(&format!(
        "\n[remote \"{}\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/{}/*\n",
        name, url, name
    ));

    fs::write(config_path, config_content)?;
    log::info!("Remote '{}' added successfully.", name);
    Ok(())
}

pub fn get_remote_url(name: &str) -> io::Result<String> {
    let config_content = fs::read_to_string(".git/config")?;
    let target_section = format!("[remote \"{}\"]", name);

    let mut in_section = false;
    for line in config_content.lines() {
        let trimmed = line.trim();
        if trimmed == target_section {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = false;
        }

        if in_section && trimmed.starts_with("url =") {
            let url = trimmed.strip_prefix("url =").unwrap().trim();
            return Ok(url.to_string());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Remote '{}' not configured in .git/config", name),
    ))
}
