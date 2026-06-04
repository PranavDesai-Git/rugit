use std::fs;
use std::io::Write;
use sha1::{Sha1,Digest};
use flate2::write::ZlibEncoder;
use flate2::Compression;

pub fn init(){
    let init_folders = vec![
        ".git",
        ".git/objects",
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

pub fn hash_objects(filepath: &str) -> std::io::Result<String> {
    let file_content = fs::read(filepath)?;
    let file_size = file_content.len();

    let mut blob = Vec::new();
    write!(blob, "blob {}\0", file_size)?;
    blob.extend_from_slice(&file_content);

    let hashed_blob = Sha1::digest(&blob);

    let hash_hex = hashed_blob
    .iter()
    .map(|byte| format!("{:02x}", byte))
    .collect::<String>();

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
