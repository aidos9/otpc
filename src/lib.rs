extern crate clipboard;
extern crate dirs;
extern crate lotp;
extern crate serde;
extern crate serde_json;
extern crate termion;
extern crate tui;
extern crate unicode_width;
mod interactive;
mod item;
mod item_storage;
mod util;
use item::{Digits, Item};
use item_storage::{storage_location, storage_location_exists};
use std::error::Error;
use std::fs;
use std::io::{stdin, stdout, Write};
use util::{is_base_32, is_number, contains_item_label};

pub fn run_interactive() {
    interactive::run();
}

pub fn run_display_code(label: &String) {
    if !storage_location_exists() {
        println!("No database file found. Please add an item first.");
        return;
    }

    match item_storage::retrieve_items(&storage_location()) {
        Ok(ref mut items) => {
            for item in items {
                if &item.label == label {
                    match item.get_code() {
                        Ok(code) => println!("{} - {}", item.label, code),
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("An error occurred when reading the database: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_remove(label: &String) {
    if !storage_location_exists() {
        println!("No database file found. Please add an item first.");
        return;
    }

    match item_storage::retrieve_items(&storage_location()) {
        Ok(ref mut items) => {
            let mut index = 0;

            while index < items.len() {
                if &items[index].label == label {
                    items.remove(index);
                } else {
                    index += 1;
                }
            }

            match item_storage::write_items(&storage_location(), items) {
                Ok(()) => println!("Successfully removed '{}' from the database.", label),
                Err(e) => {
                    eprintln!("An error occurred when writing the database: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("An error occurred when reading the database: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_list() {
    if !storage_location_exists() {
        println!("No database file found. Please add an item first.");
        return;
    }

    match item_storage::retrieve_items(&storage_location()) {
        Ok(ref mut items) => {
            if items.len() == 0 {
                println!("No items in the database.");
            }

            for item in items {
                match item.get_code() {
                    Ok(code) => println!("{} - {}", item.label, code),
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1)
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("An error occurred when reading the database: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_new() {
    let mut label = String::new();

    println!();
    while label.is_empty() {
        label = String::new();
        print!("Label: ");
        let _ = stdout().flush();
        match stdin().read_line(&mut label) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Could not retrieve user input.");
                std::process::exit(1);
            }
        }

        label = String::from(label.trim());
    }

    let mut secret = String::new();
    let mut is_valid = false;

    while !is_valid {
        secret = String::new();
        print!("Secret (base-32 formatted): ");

        let _ = stdout().flush();
        match stdin().read_line(&mut secret) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Could not retrieve user input.");
                std::process::exit(1);
            }
        }
        secret = String::from(secret.trim());
        is_valid = is_base_32(&secret);

        if !is_valid {
            eprintln!("The secret must be a base-32 string.");
        }

        is_valid = is_valid && !secret.is_empty();
    }

    let mut digits;

    loop {
        digits = String::new();
        print!("Number of digits(6/7/8, default: 6): ");

        let _ = stdout().flush();
        match stdin().read_line(&mut digits) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Could not retrieve user input.");
                std::process::exit(1);
            }
        }

        digits = String::from(digits.trim());

        if digits.is_empty() {
            digits = String::from("6");
        }

        is_valid = digits == "6" || digits == "7" || digits == "8";

        if !is_valid {
            eprintln!("The number of digits must be 6, 7 or 8.");
        } else {
            break;
        }
    }

    let mut period;

    loop {
        period = String::new();
        print!("Token period(seconds, default: 30): ");

        let _ = stdout().flush();
        match stdin().read_line(&mut period) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Could not retrieve user input.");
                std::process::exit(1);
            }
        }

        period = String::from(period.trim());

        if period.is_empty() {
            period = String::from("30");
        }

        is_valid = is_number(&period) && period != "0";

        if !is_valid {
            eprintln!("The period must be number greater than 0.");
        } else {
            break;
        }
    }

    println!("\nThe item to be added: ");
    println!("Label: {}", label);
    println!("Secret: {}", secret);
    println!("Digits: {}", digits);
    println!("Token period: {} seconds", period);

    let mut confirm = String::new();

    loop {
        print!("Add to database (y/N) ");

        let _ = stdout().flush();
        match stdin().read_line(&mut confirm) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Could not retrieve user input.");
                std::process::exit(1);
            }
        }

        confirm = String::from(confirm.trim());

        if confirm.to_lowercase() == "y" {
            break;
        } else if confirm.to_lowercase() == "n" {
            std::process::exit(0);
        }
    }

    let digits_enum: Digits;
    if digits == "8" || digits == "eight" {
        digits_enum = Digits::Eight;
    } else if digits == "7" || digits == "seven" {
        digits_enum = Digits::Seven;
    } else {
        digits_enum = Digits::Six;
    }

    let period_num: u32;

    match period.parse::<u32>() {
        Ok(d) => period_num = d,
        Err(_) => {
            eprintln!("Could not convert the supplied period into a number.");
            std::process::exit(1);
        }
    }

    let item = Item {
        label,
        secret,
        digits: digits_enum,
        split_time: period_num,
    };

    if storage_location_exists() {
        match item_storage::retrieve_items(&storage_location()) {
            Ok(ref mut items) => {
                if contains_item_label(&item.label, &items) {
                    eprintln!("An item with this label already exists.");
                    std::process::exit(1);
                }

                items.push(item);

                match item_storage::write_items(&storage_location(), items) {
                    Ok(()) => (),
                    Err(e) => {
                        eprintln!("An error occurred when writing the database: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("An error occurred when reading the database: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match item_storage::write_items(&storage_location(), &vec![item]) {
            Ok(()) => (),
            Err(e) => {
                eprintln!("An error occurred when writing the database: {}", e);
                std::process::exit(1);
            }
        }
    }

    println!("\nSuccessfully added to database.");
}

pub fn run_startup_checks() -> Option<String> {
    match dirs::home_dir() {
        Some(mut path) => {
            if !path.exists() {
                return Some(String::from("The home directory does not exist"));
            }

            path.push(".otpc");
            if path.exists() {
                return None;
            } else {
                match fs::create_dir(path) {
                    Ok(_) => return None,
                    Err(e) => {
                        return Some(format!(
                            "An error occurred whilst making the storage directory: {}",
                            e.description()
                        ))
                    }
                }
            }
        }
        None => return Some(String::from("Could not determine home directory.")),
    }
}
