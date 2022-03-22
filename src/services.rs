use std::{path::{Path, PathBuf}, io::Read};

use serde::Deserialize;
use toml;

use crate::SERVICE_MAPPING_CONFIG_PATH;

#[derive(Deserialize)]
struct ServiceIds {
    services : Vec<(u16,String)>
}

fn load_config(config : &Path) -> Result<ServiceIds,std::io::Error> {
    let mut file = std::fs::File::open(config)?;
    let mut buf = String::new();
    let _len = file.read_to_string(&mut buf)?;
    let service_ids : ServiceIds = toml::from_str(&buf)?;
    Ok(service_ids)    
}

/// Retrieve the service Ids of the services names from the configuration file
pub fn get_service_ids(config:&Path, services : Vec<& str>) -> Result<Vec<(String,u16)>, std::io::Error> {
    let service_ids = load_config(config)?;
    let res : Vec<(String,u16)> = service_ids.services.into_iter().filter_map(|(id,name)| {
        if services.contains(&name.as_str()) {
            Some((name,id))
        } else {
            None
        }
    }).collect();    
    Ok(res)
}

// get the path to the service id configuration file
pub fn get_config_path() -> Result<PathBuf,std::io::Error> {
    if let Ok(env) = std::env::var(SERVICE_MAPPING_CONFIG_PATH) {
        let config_path = PathBuf::from(env);
        if config_path.exists() {
            Ok(config_path)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Config file set in environment variable not found"))
        }
    } else {
        // No environment variable, just use the default
        let config_path = PathBuf::from(SERVICE_MAPPING_CONFIG_PATH);
        if config_path.exists() {
            Ok(config_path)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Default Config file not found"))
        }
    }
}