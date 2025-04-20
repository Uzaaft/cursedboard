mod config;
mod error;

use std::{io::Write, net::TcpStream, ops::Deref, thread, time::Duration};

use objc2::rc::{Retained, autoreleasepool};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

/// Wrapper around apple Pasteboard
struct Clipboard(Retained<NSPasteboard>);

impl Default for Clipboard {
    fn default() -> Self {
        let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
        Clipboard(pasteboard)
    }
}

impl Deref for Clipboard {
    type Target = Retained<NSPasteboard>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn main() -> Result<(), error::AppError> {
    let default_interval = Duration::from_millis(500); // TODO: Config value
    // TODO: Consider BufWriter
    // TODO: Add logic for handling ConnectionRefused
    let addr = "172.16.104.129:34254";
    let mut stream = TcpStream::connect(addr)?;

    let cp = Clipboard::default();
    let mut prev_count = 0;
    loop {
        let changecount = unsafe { cp.changeCount() };
        // If changecount is not changed, skip
        if changecount == prev_count {
            thread::sleep(default_interval);
            continue;
        }
        let text = unsafe { cp.stringForType(NSPasteboardTypeString) };
        if let Some(s) = &text {
            autoreleasepool(|pool| write_message(&stream, unsafe { s.to_str(pool) }));
        }

        prev_count = changecount;
        println!("Changecount: {prev_count}, Contents: {:?}", &text);
        // Sleep
        thread::sleep(default_interval);
    }
}

fn write_message(mut stream: &TcpStream, msg: &str) -> Result<(), error::AppError> {
    let msg_len: u64 = msg.len().try_into()?;
    let len_buffer = msg_len.to_be_bytes();
    stream.write_all(&len_buffer)?;
    let buffer = msg.as_bytes();
    stream.write_all(&buffer)?;
    Ok(())
}
