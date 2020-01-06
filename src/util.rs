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

#[cfg(test)]
mod test {
    #[test]
    pub fn test_is_base_32() {
        use super::*;
        assert!(is_base_32(&String::from("test")));
    }

    #[test]
    pub fn test_is_base_32_fail() {
        use super::*;
        assert!(!is_base_32(&String::from("abc123")));
    }

    #[test]
    pub fn test_is_base_32_c() {
        use super::*;
        assert!(is_base_32_c('c'));
    }

    #[test]
    pub fn test_is_base_32_c_fail() {
        use super::*;
        assert!(!is_base_32_c('1'));
    }

    #[test]
    pub fn test_is_number() {
        use super::*;
        assert!(is_number(&String::from("123")));
    }

    #[test]
    pub fn test_is_number_fail() {
        use super::*;
        assert!(!is_number(&String::from("123a")));
    }

    #[test]
    pub fn test_contains_white_space() {
        use super::*;
        assert!(!contains_white_space(&String::from("123")));
    }

    #[test]
    pub fn test_contains_white_space_fail_space() {
        use super::*;
        assert!(contains_white_space(&String::from("1 23a")));
    }

    #[test]
    pub fn test_contains_white_space_fail_tab() {
        use super::*;
        assert!(contains_white_space(&String::from("1\t23a")));
    }

    #[test]
    pub fn test_contains_item_label() {
        use super::*;
        use crate::Digits;
        let items = vec![
            Item {
                label: String::from("test1"),
                secret: String::from("test"),
                digits: Digits::Six,
                split_time: 30,
            },
            Item {
                label: String::from("test2"),
                secret: String::from("test2"),
                digits: Digits::Six,
                split_time: 30,
            },
        ];

        assert!(contains_item_label(&String::from("test1"), &items));
    }

    #[test]
    pub fn test_contains_item_label_fail() {
        use super::*;
        use crate::Digits;
        let items = vec![
            Item {
                label: String::from("test1"),
                secret: String::from("test"),
                digits: Digits::Six,
                split_time: 30,
            },
            Item {
                label: String::from("test2"),
                secret: String::from("test2"),
                digits: Digits::Six,
                split_time: 30,
            },
        ];

        assert!(!contains_item_label(&String::from("test3"), &items));
    }
}
