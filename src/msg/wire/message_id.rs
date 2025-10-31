use core::fmt;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

use uuid::Uuid;

pub struct Id {
    value: String,
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Used for logging; we generally only want the short version of ids
impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.shorten())
    }
}

impl Id {
    pub fn shorten(&self) -> String {
        self.value.chars().take(8).collect()
    }

    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4().to_string(),
        }
    }
}
