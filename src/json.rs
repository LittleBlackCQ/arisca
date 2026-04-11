use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub trait ToJson {
    fn to_json_string(&self) -> String;

    fn write_json(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let json = self.to_json_string();
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}
