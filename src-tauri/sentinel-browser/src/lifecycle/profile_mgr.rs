use std::path::PathBuf;

/// Manages isolated browser profiles for automation sessions
pub struct ProfileManager {
    base_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a new isolated profile directory
    pub fn create_profile(&self, name: &str) -> std::io::Result<PathBuf> {
        let profile_dir = self.base_dir.join(name);
        std::fs::create_dir_all(&profile_dir)?;
        Ok(profile_dir)
    }

    /// List existing profiles
    pub fn list_profiles(&self) -> std::io::Result<Vec<String>> {
        let mut profiles = Vec::new();
        if self.base_dir.exists() {
            for entry in std::fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        profiles.push(name.to_string());
                    }
                }
            }
        }
        Ok(profiles)
    }

    /// Delete a profile and all its data
    pub fn delete_profile(&self, name: &str) -> std::io::Result<()> {
        let profile_dir = self.base_dir.join(name);
        if profile_dir.exists() {
            std::fs::remove_dir_all(&profile_dir)?;
        }
        Ok(())
    }

    /// Get the path for the default automation profile
    pub fn default_profile(&self) -> PathBuf {
        self.base_dir.join("sentinel-default")
    }
}
