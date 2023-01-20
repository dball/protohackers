use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
pub struct Person {
    pub name: String,
}

lazy_static! {
    static ref PERSON_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{1,64}$").unwrap();
}

impl Person {
    pub fn new(name: String) -> Option<Self> {
        if PERSON_NAME.is_match(&name) {
            Some(Self { name })
        } else {
            None
        }
    }
}
