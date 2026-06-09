use byteorder::{BigEndian, ReadBytesExt};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex::encode;
use sha1::{Digest, Sha1};
use std::fs;
use std::io;
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
        return Ok(Vec::new()); // No index yet, return empty list
    }

    let mut file = fs::File::open(index_path)?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;

    // Verify signature "DIRC"
    if &header[0..4] != b"DIRC" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid index signature"));
    }

    let _version = u32::from_be_bytes(header[4..8].try_into().unwrap());
    let entry_count = u32::from_be_bytes(header[8..12].try_into().unwrap());
    
    let mut entries = Vec::new();

    for _ in 0..entry_count {
        // Read fixed fields (62 bytes total before filepath)
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

        // FIXED: Padding logic matches serialize() relative to entry start
        let entry_len_without_padding = 62 + path_len;
        let mut padding_len = 1; 
        while (entry_len_without_padding + padding_len) % 8 != 0 {
            padding_len += 1;
        }
        let mut padding_buf = vec![0u8; padding_len];
        file.read_exact(&mut padding_buf)?;

        entries.push(IndexEntry {
            ctime_secs, ctime_nanos, mtime_secs, mtime_nanos,
            dev, ino, mode, uid, gid, file_size, sha1, flags, filepath
        });
    }

    Ok(entries)
}

pub fn write_index(entries: &[IndexEntry]) -> io::Result<()> {
    let mut index_content = Vec::new();

    // 1. Header: Signature (4B), Version (4B), Entry Count (4B)
    index_content.extend_from_slice(b"DIRC");
    index_content.extend_from_slice(&2u32.to_be_bytes());
    index_content.extend_from_slice(&(entries.len() as u32).to_be_bytes());

    // 2. Entries
    for entry in entries {
        index_content.extend(entry.serialize());
    }

    // 3. Checksum: SHA-1 of everything written so far
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
