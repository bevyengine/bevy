pub mod bsn {
    #[path = "../../mod.rs"]
    pub mod fmt;

    #[path = "../../../traits.rs"]
    pub mod traits;

    #[path = "../../../types.rs"]
    pub mod types;

    #[path = "../../../parse.rs"]
    pub mod parse;

    #[path = "../../../codegen.rs"]
    pub mod codegen;
}

use clap::Parser;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    io::{self, Read},
    process,
};
use syn::{visit::Visit, File};

use bsn::fmt::{col_to_offset, BsnVisitor};

/// A CLI formatter for `bsn!` macros.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The file to format. If omitted, reads from stdin and prints to stdout.
    file: Option<String>,

    /// Write the formatted output back to the file.
    /// Ignored if reading from stdin.
    #[arg(short, long)]
    write: bool,
}

fn main() {
    let args = Args::parse();
    let is_stdin = args.file.is_none();

    let src = if let Some(path) = &args.file {
        fs::read_to_string(path).unwrap_or_else(|_| {
            eprintln!("Failed to read file: {}", path);
            process::exit(1);
        })
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap_or_else(|_| {
            eprintln!("Failed to read from stdin");
            process::exit(1);
        });
        buffer
    };

    let ast: File = syn::parse_file(&src).unwrap_or_else(|_| {
        if is_stdin {
            print!("{}", src);
        } else {
            eprintln!("Failed to parse Rust file: Invalid syntax.");
        }
        process::exit(1);
    });

    let mut visitor = BsnVisitor {
        source: &src,
        edits: Vec::new(),
    };

    visitor.visit_file(&ast);

    if is_stdin {
        let mut output = src.clone();
        if !visitor.edits.is_empty() {
            visitor
                .edits
                .sort_by_key(|b| std::cmp::Reverse(b.start_line));

            for edit in &visitor.edits {
                let start_byte = col_to_offset(&output, edit.start_line, edit.start_col);
                let end_byte = col_to_offset(&output, edit.end_line, edit.end_col);

                output.replace_range(start_byte..end_byte, &edit.new_text);
            }
        }

        print!("{}", output);
        return;
    }

    let path = args
        .file
        .expect("Should have a value if 'is_stdin' is false.");

    if visitor.edits.is_empty() {
        println!("\x1b[1;32mNo formatting changes needed in {}\x1b[0m", path);
        return;
    }

    if args.write {
        visitor
            .edits
            .sort_by_key(|b| std::cmp::Reverse(b.start_line));

        let mut output = src.clone();
        let edit_count = visitor.edits.len();
        for edit in visitor.edits {
            let start_byte = col_to_offset(&output, edit.start_line, edit.start_col);
            let end_byte = col_to_offset(&output, edit.end_line, edit.end_col);

            output.replace_range(start_byte..end_byte, &edit.new_text);
        }

        fs::write(&path, output).expect("Failed to write to file");
        println!(
            "\x1b[1;32mbsn! fmt: formatted {} block(s) in {}\x1b[0m",
            edit_count, path
        );
    } else {
        println!(
            "bsn! fmt: found {} unformatted block(s) in {}\n",
            visitor.edits.len(),
            path
        );

        for edit in &visitor.edits {
            println!(
                "\x1b[1;36m▶ Location: line {}, col {}\x1b[0m",
                edit.start_line, edit.start_col
            );

            let diff = TextDiff::from_lines(&edit.original_text, &edit.new_text);

            for group in diff.grouped_ops(3) {
                for op in group {
                    for change in diff.iter_inline_changes(&op) {
                        let (sign, color) = match change.tag() {
                            ChangeTag::Delete => ("-", "\x1b[31m"), // red
                            ChangeTag::Insert => ("+", "\x1b[32m"), // green
                            ChangeTag::Equal => (" ", "\x1b[90m"),  // gark Gray
                        };

                        print!("{}{} ", color, sign);
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
            }
            println!("--------------------------------------------------");
        }
        println!("\nRun with \x1b[1m--write\x1b[0m to apply these changes in-place.");
    }
}
