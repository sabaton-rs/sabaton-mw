use std::{env, fs, os::unix::prelude::PermissionsExt, path::Path};
use tracing::{debug, error};
use crate::error::MiddlewareError;

pub fn create_home_directory_if_required(application_name: &String) -> Result<(), MiddlewareError> {
    let name = "HOME";
    match env::var(name) {
        Ok(v) => debug!("HOME environment variable is set to {}. Using it", name),
        Err(e) => {
            let path = Path::new("/data/home/");
            let path = path.join(&application_name);
            if !std::path::Path::new(&path).exists() {
                std::fs::create_dir(&path)?;
            };
            let metadata = fs::metadata(path)?;
            let mut permissions = metadata.permissions();

            permissions.set_mode(0o644); // Read/write for owner and read for others.
        }
    }

    Ok(())
}
