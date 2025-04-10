use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use shared::clipboard::ClipboardProvider;
use std::sync::mpsc;

pub struct MacOSClipboardProvider {
    command_tx: mpsc::Sender<ClipboardCommand>,
}

enum ClipboardCommand {
    GetText(mpsc::Sender<Result<String, String>>),
    SetText(String),
    CheckChanged(mpsc::Sender<Option<String>>),
}

impl MacOSClipboardProvider {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, command_rx) = mpsc::channel();

        // Spawn a thread to handle clipboard operations
        std::thread::spawn(move || {
            let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
            let mut last_changecount = 0;
            let mut last_content = String::new();

            loop {
                match command_rx.try_recv() {
                    Ok(ClipboardCommand::GetText(response_tx)) => {
                        let text = unsafe { pasteboard.stringForType(NSPasteboardTypeString) };
                        let result = match text {
                            Some(s) => {
                                let content =
                                    autoreleasepool(|pool| unsafe { s.to_str(pool).to_string() });
                                Ok(content)
                            }
                            None => Ok(String::new()),
                        };
                        let _ = response_tx.send(result);
                    }
                    Ok(ClipboardCommand::SetText(text)) => {
                        autoreleasepool(|_pool| unsafe {
                            pasteboard.clearContents();
                            let ns_string = NSString::from_str(&text);
                            pasteboard.setString_forType(&ns_string, NSPasteboardTypeString);
                        });
                        last_changecount = unsafe { pasteboard.changeCount() };
                        last_content = text;
                    }
                    Ok(ClipboardCommand::CheckChanged(response_tx)) => {
                        let changecount = unsafe { pasteboard.changeCount() };
                        let mut result = None;

                        if changecount != last_changecount {
                            last_changecount = changecount;

                            let text = unsafe { pasteboard.stringForType(NSPasteboardTypeString) };
                            if let Some(s) = text {
                                let content =
                                    autoreleasepool(|pool| unsafe { s.to_str(pool).to_string() });

                                if content != last_content && !content.is_empty() {
                                    last_content = content.clone();
                                    result = Some(content);
                                }
                            }
                        }

                        let _ = response_tx.send(result);
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // Sleep briefly to avoid busy waiting
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        });

        Ok(Self { command_tx })
    }
}

impl ClipboardProvider for MacOSClipboardProvider {
    fn get_text(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.command_tx
            .send(ClipboardCommand::GetText(response_tx))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        response_rx
            .recv()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
            .map_err(|e| e.into())
    }

    fn set_text(&mut self, text: String) -> Result<(), Box<dyn std::error::Error>> {
        self.command_tx
            .send(ClipboardCommand::SetText(text))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn check_changed(&mut self) -> Option<String> {
        let (response_tx, response_rx) = mpsc::channel();
        if self
            .command_tx
            .send(ClipboardCommand::CheckChanged(response_tx))
            .is_err()
        {
            return None;
        }
        response_rx.recv().unwrap_or(None)
    }
}
