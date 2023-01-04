use audiotags::{AudioTag, Tag};
use fs_err as fs;
use jwalk::{ WalkDir, DirEntry };
use std::{env, io, path::Path, path::PathBuf};

#[allow(dead_code)]
#[derive(Debug)]
struct CopyOp {
    from: PathBuf,
    to: PathBuf,
}

fn _copy_file_and_make_dirs(tag: Box<dyn AudioTag>, from: &Path, to: &Path) -> io::Result<()> {
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

fn should_show(entry: &Result<DirEntry<((), ())>, jwalk::Error>, dest: &PathBuf) -> bool {
    let entry = entry.as_ref().unwrap();
    let hidden = entry.file_name().to_str().map(|s| s.starts_with('.')).unwrap_or(false);
    let entry_in_dest = Path::new(entry.parent_path()).starts_with(dest);
    !(hidden || entry_in_dest)
}

fn analyze_dir(op_log: &mut Vec<CopyOp>, current_dir: PathBuf, dest: PathBuf) {
    let walker_dest = dest.clone();
    let walker = WalkDir::new(current_dir).process_read_dir(move |_depth, _path, _state, children| {
        children.retain(|entry| should_show(entry, &walker_dest));
    });
    for entry in walker {
        // Try to copy file to dest if music file.
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            // Tag parsing errors on files without a dot.
            if !entry.file_name().to_str().unwrap().contains('.') {
                continue;
            }
            // Try to read music metadata.
            let tag = Tag::new().read_from_path(entry.file_name());
            if let Ok(_tag) = tag {
                let to = dest.join(entry.file_name());
                println!("Queued copy from {:?} to {:?}", entry.path(), &to);
                op_log.push(CopyOp { from: entry.path(), to });
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
    let mut op_log: Vec<CopyOp> = vec![];
    analyze_dir(&mut op_log, current_dir, songs_dir);
    println!("{:?}", &op_log);
}
