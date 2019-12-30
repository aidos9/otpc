use crate::item::Item;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub fn write_items(path: &String, items: &Vec<Item>) -> Result<(), String> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(file) => {
            let writer = BufWriter::new(file);
            match serde_json::to_writer(writer, items) {
                Ok(_) => return Ok(()),
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
