mod error;
mod config;

use std::{io::Write, net::TcpStream, ops::Deref, thread, time::Duration};

use objc2::rc::Retained;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

/// Wrapper around apple Pasteboard
struct Clipboard(Retained<NSPasteboard>);

impl Default for Clipboard{
    fn default() -> Self{
        let pasteboard =unsafe{ NSPasteboard::generalPasteboard()};
        Clipboard (
            pasteboard
        )
    }
}

impl Deref for Clipboard{
    type Target = Retained<NSPasteboard>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn main() -> Result<(), error::AppError > {
    let mut stream = TcpStream::connect("dev.bobr:34254")?;

    let cp = Clipboard::default();
    let default_interval = Duration::from_millis(500); // TODO: Config value
    let mut prev_count = 0;
    loop {
        let changecount = unsafe{ cp.changeCount()};
        // If changecount is not changed, skip
        if changecount == prev_count{
            thread::sleep(default_interval);
            continue;
        }
        let text = unsafe { cp.stringForType(NSPasteboardTypeString) };
        if let Some(s) = &text {
            stream.write_all(&s.to_string().into_bytes())?;
        }

        prev_count = changecount;
        println!("Changecount: {prev_count}, Contents: {:?}", &text);
        // Sleep
        thread::sleep(default_interval);
    }
}
