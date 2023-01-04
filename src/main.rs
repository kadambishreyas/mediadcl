use audiotags::Tag;
use fs_err as fs;
use jwalk::{DirEntry, WalkDir};
use rayon::prelude::*;
use std::{env, ffi::OsString, path::Path, path::PathBuf};

#[derive(Debug, PartialEq)]
enum CopyStatus {
    NotYet,
    Executing,
    Success,
    Failure,
}

#[derive(Debug)]
struct CopyOp {
    src: PathBuf,
    dest: PathBuf,
    status: CopyStatus,
}

impl CopyOp {
    fn new(src: PathBuf, dest: PathBuf) -> CopyOp {
        CopyOp { src, dest, status: CopyStatus::NotYet }
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
    let walker =
        WalkDir::new(current_dir).process_read_dir(move |_depth, _path, _state, children| {
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
                op_log.push(CopyOp::new(entry.path(), to));
            }
        }
    }
}

fn can_write_new(path: &Path) -> bool {
    let exists = path.try_exists().is_ok() && path.try_exists().unwrap();
    let parent = path.parent().expect("Could not find parent.");
    let may_create =
        parent.metadata().is_ok() && !parent.metadata().unwrap().permissions().readonly();
    !exists && may_create
}

fn copy_files(op_log: &mut Vec<CopyOp>) {
    op_log.par_iter_mut().for_each(|op| {
        op.status = CopyStatus::Executing;

        // Make dest if it doesn't exist.
        assert!(op.dest.file_name().is_some());
        if let Some(p) = op.dest.parent() {
            if can_write_new(p) {
                println!("Creating directory {:?}.", p);
                fs::create_dir_all(p).expect("Could not move file to desired location.");
            }
        }

        // Ensure copy is possible & does not overwrite.
        if !can_write_new(&op.dest) {
            op.status = CopyStatus::Failure;
        } else {
            // Try to copy file.
            match fs::copy(&op.src, &op.dest) {
                Ok(_bytes_copied) => {
                    println!("Copied {:?} to {:?}", &op.src, &op.dest);
                    op.status = CopyStatus::Success;
                },
                Err(e) => {
                    println!("Copying {:?} failed. {:?}", &op.src, e);
                    op.status = CopyStatus::Failure;
                },
            }
        }
    });
}

fn main() {
    // Get current directory.
    let current_dir = env::current_dir().unwrap_or_else(|_| {
        println!("Fatal error: current directory could not be found.");
        std::process::exit(1);
    });

    // Analyze new locations of all files to be moved.
    let songs_dir = current_dir.join("songs");
    let mut op_log: Vec<CopyOp> = vec![];
    analyze_dir(&mut op_log, current_dir, songs_dir);

    // Copy files.
    copy_files(&mut op_log);

    // Status report.
    let op_log: Vec<OsString> = op_log
        .into_iter()
        .filter(|op| op.status != CopyStatus::Success)
        .map(|op| op.src.into_os_string())
        .collect();
    if op_log.is_empty() {
        println!("All files copied!");
    } else {
        println!("List of uncopied files: {:?}", op_log);
    }
}
