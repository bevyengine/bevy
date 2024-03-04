use std::{fmt, panic::PanicInfo};

use backtrace::{Backtrace, BacktraceFmt, BytesOrWideString, PrintFmt};

pub fn panic_hook(info: &PanicInfo) {
    // The current implementation always returns `Some`.
    let location = info.location().unwrap();
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<dyn Any>",
        },
    };
    eprintln!("Panic at {location}:\n{msg}");

    eprintln!("{}", DisplayBacktrace);
}

struct DisplayBacktrace;

impl fmt::Display for DisplayBacktrace {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backtrace = Backtrace::new();

        let style = match std::env::var("RUST_BACKTRACE") {
            Ok(s) if s == "full" => PrintFmt::Full,
            Ok(_) => PrintFmt::Short,
            Err(_) => return Ok(()),
        };

        // When printing paths we try to strip the cwd if it exists, otherwise
        // we just print the path as-is. Note that we also only do this for the
        // short format, because if it's full we presumably want to print
        // everything.
        let cwd = std::env::current_dir();
        let mut print_path = move |fmt: &mut fmt::Formatter<'_>, path: BytesOrWideString<'_>| {
            let path = path.into_path_buf();
            if style == PrintFmt::Short {
                if let Ok(cwd) = &cwd {
                    if let Ok(suffix) = path.strip_prefix(cwd) {
                        return fmt::Display::fmt(&suffix.display(), fmt);
                    }
                }
            }
            fmt::Display::fmt(&path.display(), fmt)
        };
        let mut f = BacktraceFmt::new(fmt, style, &mut print_path);

        f.add_context()?;
        if style == PrintFmt::Full {
            for frame in backtrace.frames() {
                f.frame().backtrace_frame(frame)?;
            }
        } else {
            let mut interesting_frame = false;
            'frame: for frame in backtrace.frames() {
                for symbol in frame.symbols() {
                    if let Some(sym) = symbol.name().and_then(|s| s.as_str()) {
                        // TODO: How to mark beginning of stacktrace?
                        // Note: Use SymbolName's Display to demangle if there are multiple path segments
                        if sym.contains("__rust_begin_short_backtrace") | sym.contains("bevy_ecs") {
                            interesting_frame = false;
                            continue 'frame;
                        }
                        if sym.contains("__rust_end_short_backtrace") {
                            interesting_frame = true;
                            continue 'frame;
                        }
                    }
                }
                if interesting_frame {
                    f.frame().backtrace_frame(frame)?;
                }
            }
        }
        f.finish()?;
        Ok(())
    }
}
