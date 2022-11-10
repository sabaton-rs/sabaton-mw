use std::{path::Path, env};
use anyhow::Result;

pub fn create_directory(name:&String)->Result<()>
{
    let name = "HOME";
    match env::var(name) {
        Ok(v) => println!("{}: {}", name, v),
        Err(e) => {let path = Path::new("/home/devuser/");
        let path=path.join(&name);
        if !std::path::Path::new(&path).exists() {
            std::fs::create_dir(&path)?;
        };}
    }
    
        Ok(())
}