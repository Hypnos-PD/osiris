use std::path::PathBuf;
use std::fs;

pub trait ScriptLoader {
    fn load_script(&self, name: &str) -> Option<String>;
}

pub struct FileSystemLoader {
    base_path: PathBuf,
}

impl FileSystemLoader {
    pub fn new(base_path: PathBuf) -> Self {
        FileSystemLoader { base_path }
    }
}

impl ScriptLoader for FileSystemLoader {
    fn load_script(&self, name: &str) -> Option<String> {
        let full_path = self.base_path.join(name);
        if !full_path.exists() {
            return None;
        }
        match fs::read_to_string(full_path) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_existing_script() {
        let loader = FileSystemLoader::new(Path::new("../external/ygopro/script").to_path_buf());
        let s = loader.load_script("constant.lua");
        assert!(s.is_some());
        let content = s.unwrap();
        assert!(content.contains("TYPE_MONSTER"));
    }
}
