use std::{
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

use crate::error::AppError;

struct Config {
    /// Check interval in ms
    check_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            check_interval: 500,
        }
    }
}

impl Config {
    // TODO: THis is a placeholder for now. Finish this at a later point
    fn read(path: PathBuf) -> Result<Self, AppError> {
        if !path.is_file() {
            // TODO: Log this
            return Ok(Self::default());
        }
        let file = File::open(path)?;

        let lines = io::BufReader::new(file).lines();
        lines.map_while(Result::ok).map(|r| dbg!(r));

        Ok(Self {
            check_interval: Default::default(),
        })
    }
}
