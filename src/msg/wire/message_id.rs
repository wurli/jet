/*
 * message_id.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use core::fmt;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Id {
    /// The actual unique Id
    value: String,
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id { value: s }
    }
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        id.value
    }
}

/// Used for logging; we generally only want the short version of ids
impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{}>", self.value.chars().take(7).collect::<String>())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl Id {
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4().to_string(),
        }
    }

    pub fn unparented() -> Self {
        Self {
            value: String::from("unparented"),
        }
    }
}
