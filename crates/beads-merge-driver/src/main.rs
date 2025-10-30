use std::collections::BTreeSet;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process;

fn read_lines(path: &str, set: &mut BTreeSet<String>) -> io::Result<()> {
    let file = File::open(path);
    let file = match file {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.is_empty() {
            set.insert(line);
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: beads-merge-driver <base> <local> <remote>");
        process::exit(1);
    }

    let mut lines = BTreeSet::new();

    for path in &args[1..=3] {
        if let Err(err) = read_lines(path, &mut lines) {
            eprintln!("error reading {path}: {err}");
            process::exit(1);
        }
    }

    for line in lines {
        println!("{line}");
    }
}
