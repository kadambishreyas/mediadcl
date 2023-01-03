use audiotags::{AudioTag, Tag};
use fs_err as fs;
use std::{env, io, path::Path, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

fn copy_file_and_make_dirs(tag: Box<dyn AudioTag>, from: &Path, to: &Path) -> io::Result<()> {
    // Make directories.
    if let Some(p) = to.parent() {
        fs::create_dir_all(p)?
    }

    // Perform copy.
    let copy_result = fs::copy(from, to);
    match copy_result {
        Ok(_bytes_copied) => {
            println!(
                "{:?} - {:?} copied!",
                tag.album_artist().unwrap_or("Unknown Artist"),
                tag.title().unwrap_or("Unknown Title")
            );
            Ok(())
        },
        Err(e) => Err(e),
    }
}

fn should_hide(entry: &DirEntry) -> bool {
    entry.file_name().to_str().map(|s| s.starts_with('.')).unwrap_or(false)
}

fn copy_dir(current_dir: PathBuf, dest: PathBuf) {
    // Recursively walk while skipping hidden files.
    let walker = WalkDir::new(current_dir.as_path()).min_depth(1).into_iter().filter_entry(|entry| !should_hide(entry));
    // let pretraversal_list: Vec<DirEntry> = walker
    //     .filter_entry(|entry| !should_hide(entry))
    //     .map(|r| r.unwrap())
    //     .collect();
    for entry in walker {
        // Ignore if file is in dest.
        let entry = entry.unwrap();
        if entry.path().starts_with(dest.as_path()) {
            println!("File in destination! Entry path: {:?}, dest path: {:?}", entry.path(), dest);
            continue;
        }

        // Try to copy file to dest if music file.
        if entry.file_type().is_file() {
            // Tag parsing errors on files without a dot.
            if !entry.file_name().to_str().unwrap().contains('.') {
                continue;
            }

            // Try to read music metadata.
            let tag = Tag::new().read_from_path(entry.file_name());
            if let Ok(tag) = tag {
                println!("Copying file from {:?} to {:?}", entry.path(), dest.join(entry.file_name()).as_path());
                copy_file_and_make_dirs(tag, entry.path(), dest.join(entry.file_name()).as_path())
                    .unwrap_or_else(|e| {
                        println!("Error when copying file: {:?}", e);
                    });
            }
        }
    }
}

fn main() {
    // Get current directory.
    let current_dir = env::current_dir().unwrap_or_else(|_| {
        println!("Fatal error: current directory could not be found.");
        std::process::exit(1);
    });

    // Iterate through current directory.
    let songs_dir = current_dir.join("songs");
    copy_dir(current_dir, songs_dir);
}
