use std::fs;

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
