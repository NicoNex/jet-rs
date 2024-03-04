use clap::Parser;
use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use std::fs::File;
use std::io::{self, Read, Write};
use walkdir::{DirEntry, WalkDir};

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Options {
    pattern: String,
    replacement: String,
    path: String,
    /// Add a glob the file names must match to be edited.
    #[arg(short, long)]
    glob: Option<String>,
    /// Print to stdout instead of writing each file.
    #[arg(short = 'p', long = "print")]
    to_stdout: bool,
    /// Verbose, explain what is being done.
    #[arg(short, long)]
    verbose: bool,
    /// Max depth in a directory tree.
    #[arg(short = 'l', long = "level", default_value_t = -1)]
    depth: i32,
    /// Includes hidden files (starting with a dot).
    #[arg(short = 'a', long = "all")]
    include_hidden: bool,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn process_file(
    entry: walkdir::DirEntry,
    re: &regex::Regex,
    replacement: &String,
    verbose: bool,
    to_stdout: bool,
) {
    let path = entry.path();

    if let Ok(mut file) = File::open(&path) {
        let mut cnt = String::new();
        if let Err(err) = file.read_to_string(&mut cnt) {
            if verbose {
                eprintln!("error: failed to read file {:?}: {}", path, err);
            }
            return;
        }

        let modified = re.replace_all(&cnt, replacement);

        if to_stdout {
            println!("{}", modified);
            return;
        }

        if let Ok(mut modified_file) = File::create(&path) {
            if let Err(err) = modified_file.write_all(modified.as_bytes()) {
                eprintln!("error: failed to write to file {:?}: {}", path, err);
                return;
            }
            if verbose {
                println!("{:?} modified", path);
            }
        } else {
            eprintln!("error: could not override file {:?}", path);
        }
    }
}

fn process_stdin(re: &regex::Regex, replacement: &String) {
    let mut cnt = String::new();
    let mut stdin = io::stdin();

    if let Err(err) = stdin.read_to_string(&mut cnt) {
        eprintln!("error: failed to read stdin: {}", err);
        return;
    }
    print!("{}", re.replace_all(&cnt, replacement));
}

fn main() {
    let opts = Options::parse();
    let re = Regex::new(opts.pattern.as_str()).unwrap();

    if opts.path == "-" {
        return process_stdin(&re, &opts.replacement);
    }

    let pattern = Pattern::new(opts.glob.as_deref().unwrap_or("*")).expect("Invalid glob pattern");
    let walker = WalkDir::new(String::from(opts.path)).into_iter();

    walker
        .filter_entry(|e| is_hidden(e) || !opts.include_hidden)
        .filter_map(Result::ok)
        .filter(|e| pattern.matches(e.path().to_string_lossy().as_ref()))
        .filter(|e| opts.depth < 0 || e.depth() <= opts.depth as usize)
        .filter(|e| !e.path().is_dir())
        .par_bridge()
        .for_each(|e| process_file(e, &re, &opts.replacement, opts.verbose, opts.to_stdout));
}
