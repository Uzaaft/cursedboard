mod error;
mod config;

use std::{ops::Deref, thread, time::Duration};

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
    let cp = Clipboard::default();
    let default_interval = Duration::from_millis(500); // TODO: Config value

    loop {
        thread::sleep(default_interval);
        let changecount = unsafe{ cp.changeCount()};
        let text = unsafe { cp.stringForType(NSPasteboardTypeString) };
        println!("Changecount: {changecount}, Contents: {text:?}");
    }
    Ok(())
}
