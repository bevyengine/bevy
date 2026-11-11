#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "Working with stdout and stderr is needed for better tooling & IDE integration"
)]
//! CLI BSN Formatter

use bsnfmt::{types::*, utils::col_to_offset};
use clap::Parser;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    process,
};
use syn::{visit::Visit, File};

/// A CLI formatter for `bsn!` macros.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The file to format. If omitted, reads from stdin and prints to stdout.
    files: Vec<PathBuf>,

    /// Write the formatted output back to the file.
    /// Ignored if reading from stdin.
    #[arg(short, long)]
    write: bool,
}

fn stdin() {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap_or_else(|_| {
        eprintln!("Failed to read from stdin");
        process::exit(1);
    });

    let Some((_, out)) = format(&buffer) else {
        // No changes are needed
        return;
    };

    println!("{}", out.trim());
}

fn format_files(files: Vec<PathBuf>, check: bool) {
    fn inner_format(path: PathBuf, check: bool) {
        let path = path.canonicalize().unwrap();
        let src = fs::read_to_string(&path).unwrap_or_else(|_| {
            eprintln!("Failed to read file: {}", path.display());
            process::exit(1);
        });

        let Some((visitor, out)) = format(&src) else {
            // No changes are needed
            return;
        };

        if check {
            println!("Diff in {}", path.display());
            for edit in &visitor.edits {
                let diff = TextDiff::from_lines(&edit.original_text, &edit.new_text);
                for change in diff
                    .grouped_ops(3)
                    .iter()
                    .flatten()
                    .flat_map(|op| diff.iter_inline_changes(op))
                {
                    let (sign, color) = match change.tag() {
                        ChangeTag::Delete => ("-", "\x1b[31m"), // red
                        ChangeTag::Insert => ("+", "\x1b[32m"), // green
                        ChangeTag::Equal => (" ", "\x1b[90m"),  // gark Gray
                    };

                    print!("{}{}", color, sign);

                    for (emphasized, value) in change.iter_strings_lossy() {
                        if emphasized {
                            // bold intra-line changes
                            print!("\x1b[1m{}\x1b[22m", value);
                        } else {
                            print!("{}", value);
                        }
                    }
                    // reset color
                    print!("\x1b[0m");
                }
            }
        } else {
            fs::write(path, out).expect("Failed to write to file");
        }
    }

    for file in files {
        inner_format(file, check);
    }
}

fn format(input: &str) -> Option<(BsnVisitor<'_>, String)> {
    let ast: File = syn::parse_file(input).unwrap_or_else(|_| {
        eprintln!("Failed to parse Rust file: Invalid syntax.");
        process::exit(1);
    });

    let mut visitor = BsnVisitor {
        source: input,
        edits: Vec::new(),
    };

    visitor.visit_file(&ast);

    if visitor.edits.is_empty() {
        return None;
    }

    let mut out = input.to_owned();

    visitor
        .edits
        .sort_by_key(|b| std::cmp::Reverse(b.start_line));

    for edit in &visitor.edits {
        let start_byte = col_to_offset(&out, edit.start_line, edit.start_col);
        let end_byte = col_to_offset(&out, edit.end_line, edit.end_col);

        out.replace_range(start_byte..end_byte, &edit.new_text);
    }

    Some((visitor, out))
}

fn main() {
    let args = Args::parse();

    if args.files.is_empty() {
        return stdin();
    }

    format_files(args.files, !args.write);
}
