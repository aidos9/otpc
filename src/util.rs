use crate::item::Item;

const BASE_32_ALPHABET: &'static str = "abcdefghijklmnopqrstuvwxyz234567";

pub fn is_base_32(str: &String) -> bool {
    for c in str.chars() {
        if !BASE_32_ALPHABET.contains(c) {
            return false;
        }
    }

    return true;
}

pub fn is_base_32_c(c: char) -> bool {
    return BASE_32_ALPHABET.contains(c);
}

pub fn is_number(str: &String) -> bool {
    for c in str.chars() {
        if !c.is_numeric() {
            return false;
        }
    }

    return true;
}

pub fn contains_white_space(str: &String) -> bool {
    for c in str.chars() {
        if c.is_whitespace() {
            return true;
        }
    }

    return false;
}

pub fn contains_item_label(label: &String, items: &Vec<Item>) -> bool {
    for ref lbl in items.into_iter().map(|item| item.label.clone()) {
        if lbl == label {
            return true;
        }
    }

    return false;
}
