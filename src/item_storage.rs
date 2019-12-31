use crate::item::Item;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

pub fn storage_location_exists() -> bool {
    return Path::new(&storage_location()).exists();
}

pub fn storage_location() -> String {
    match dirs::home_dir() {
        Some(mut path) => {
            path.push(".otpc");
            path.push("items.json");
            return String::from(path.to_str().unwrap());
        }
        None => return String::new(),
    }
}

pub fn write_items(path: &String, items: &Vec<Item>) -> Result<(), String> {
    if Path::new(path).exists() {
        match std::fs::remove_file(path) {
            Ok(_) => (),
            Err(_) => return Err(String::from("Could not remove the old file.")),
        }
    }

    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(file) => {
            let mut writer = BufWriter::new(file);
            match serde_json::to_string(items) {
                Ok(s) => match writer.write_all(s.as_bytes()) {
                    Ok(_) => {
                        let _ = writer.flush();
                        return Ok(());
                    }
                    Err(e) => return Err(String::from(e.description())),
                },
                Err(e) => return Err(String::from(e.description())),
            }
        }
        Err(e) => return Err(String::from(e.description())),
    }
}

pub fn retrieve_items(path: &String) -> Result<Vec<Item>, String> {
    if !Path::new(path).exists() {
        return Err(String::from("File does not exist"));
    }

    match OpenOptions::new().create(false).read(true).open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(items) => return Ok(items),
                Err(e) => return Err(String::from(e.description())),
            }
        }
        Err(e) => return Err(String::from(e.description())),
    }
}
