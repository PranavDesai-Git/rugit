use std::fs;
use std::io::Write;
use sha1::{Sha1,Digest};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex::encode;
use std::time::UNIX_EPOCH;
use std::io;

pub fn init(){
    let init_folders = vec![
        ".git",
        ".git/objects",
        ".git/refs",
        ".git/refs/heads",
        ".git/refs/tags"
    ];
    let init_files = vec![
        (".git/HEAD", 
         "ref: refs/heads/main\n"),
        (".git/config", 
         "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n"),
        (".git/description",
         "Unnamed repository; edit this file 'description' to name the repository.\n"),
    ];
    for path in init_folders{
        match fs::create_dir_all(path){
            Ok(_) => log::info!("{} created", path),
            Err(e) => {
                log::error!("init err @ {}:{}",path, e);
                return;
            }
        }
    }
    for (path, content) in init_files {
        match fs::write(path, content){
            Ok(_) => log::info!("{} created", path),
            Err(e) => {
                log::error!("init err: {}", e);
                return;
            }
        }
    }
    log::info!("git initialized");
}

pub fn hash_object(filepath: &str) -> std::io::Result<[u8;20]> {
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

#[derive(Debug,Default)]
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

impl IndexEntry{
    pub fn from_file(filepath: &str) -> Result<Self, io::Error>{
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
            entry.mode = if unix_mode & 0o111 != 0 { 0o100755 } else { 0o100644 };
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
            
            entry.mode = if metadata.permissions().readonly() { 0o100755 } else { 0o100644 }; 
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
        //padding
        bytes.push(0);
        let padding = (8 - (bytes.len() % 8)) % 8;
        bytes.extend(std::iter::repeat(0).take(padding));

        bytes
    }
}

pub fn stage_file(filepath: &str){
}
