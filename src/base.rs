use std::fs;
use std::io::Write;
use sha1::{Sha1,Digest};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hex::encode;

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

    Ok(hash_hex)
}

pub fn stage_file(filepath: &str){
    /* Pseudo code
     * let index_file
     * let index_path = ".git/index"
     * match fs::read(".git/index"){
     *      Ok(bytes) => {
     *          index_file = bytes;
     *      }
     *      Err(_)=>{
     *          index_file = File::create(file_path)?
     *      }
     * }
     * let hash_hex = hash_object(filepath)?;
     * [GRAB METADATA]
     * let metadata =  fs::metadata(path)?;
     * let mut ctime_seconds:u32 = 0;
     * let mut ctime_nanosecs:u32 = 0;
     * let mut mtime_seconds:u32 = 0;
     * let mut mtime_nanosecs:u32 = 0;
     * let mut dev:u32 = 0;
     * let mut ino: u32 = 0;
     * let mut mode:u32 = 0;
     * let mut uid:u32 = 0;
     * let mut gid:u32 = 0;
     * let mut file_size:u32 = 0;
     * let mut hash: [u8; 20] = [0; 20];
     * let mut flags:u16 = 0;
     * let fp = filepath;
     * let padding[u8; 8] = [0;8];
     * file_size = min(metadata.len(), u32::MAX as u64) as u32;
     * if let Ok(ctime) = metadata.created(){
     *      if let Ok(duration) = ctime.duration_since(SystemTime::UNIX_EPOCH){
     *          ctime_seconds = duration.as_secs() as u32;
     *          ctime_nanosecs = duration.subsec_nanos();
     *      }
     * }
     * if let Ok(mtime) = metadata.modified(){
     *      if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH){
     *          mtime_seconds = duration.as_secs() as u32;
     *          mtime_nanosecs = duration.subsec_nanos();
     *      }
     * }
     * #[cfg(unix)]
     * {
     *      use std::os::unix::fs::MetadataExt;
     *      use std::os::unix::fs::PermissionsExt;
     *      dev = metadata.dev() as u32;
     *      ino = metadata.ino() as u32;
     *      uid = metadata.uid();
     *      gid = metadata.gid();
     *      mode = metadata.permissions().mode();
     * }
     * #[cfg(windows)]
     * {
     *      use std::os::windows::fs::MetadataExt;
     *      if let Some(sn) = metadata.volume_serial_number(){
     *          dev = sn;
     *      }
     *      if let Some(fi) = metadata.file_index() {
     *          ino = file_index as u32;
     *      }
     *      if metadata.permissions().readonly() {
     *          mode = 1;
     *      }
     *      else{
     *          mode = 0;
     *      }
     * }
     * CHECK LIST
     * IF EXISTS -> UPDATE
     * NO? -> ADD
     * CLOSE INDEX FILE
     * */
}
