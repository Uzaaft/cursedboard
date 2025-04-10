use arboard::Clipboard;
use shared::clipboard::ClipboardProvider;

pub struct LinuxClipboardProvider {
    clipboard: Clipboard,
    last_content: String,
}

impl LinuxClipboardProvider {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let clipboard = Clipboard::new()?;
        Ok(Self {
            clipboard,
            last_content: String::new(),
        })
    }
}

impl ClipboardProvider for LinuxClipboardProvider {
    fn get_text(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        self.clipboard
            .get_text()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn set_text(&mut self, text: String) -> Result<(), Box<dyn std::error::Error>> {
        self.last_content = text.clone();
        self.clipboard
            .set_text(text)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn check_changed(&mut self) -> Option<String> {
        match self.get_text() {
            Ok(current) => {
                if current != self.last_content && !current.is_empty() {
                    self.last_content = current.clone();
                    Some(current)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
