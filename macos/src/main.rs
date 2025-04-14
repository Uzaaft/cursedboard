mod error;
mod config;

use std::{ops::Deref, time::Duration};

use objc2::rc::Retained;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use tokio::time;

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), error::AppError > {
    let cp = Clipboard::default();
    let default_interval = Duration::from_millis(500); // TODO: Config value
    let mut interval = time::interval(default_interval);

    loop {
        interval.tick().await;
        let changecount = unsafe{ cp.changeCount()};
        let text = unsafe { cp.stringForType(NSPasteboardTypeString) };
        println!("Changecount: {changecount}, Contents: {text:?}");
    }
    Ok(())
}
