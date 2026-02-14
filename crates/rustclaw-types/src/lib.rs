mod message;
mod provider;
mod user;
mod events;

pub use message::*;
pub use provider::*;
pub use user::*;
pub use events::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for various entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id<T>(pub Uuid, pub std::marker::PhantomData<T>);

impl<T> Id<T> {
    pub fn new() -> Self {
        Self(Uuid::new_v4(), std::marker::PhantomData)
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
