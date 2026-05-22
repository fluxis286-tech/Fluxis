#![allow(dead_code)]
// FLUXIS — dop/actors.rs
// Actor brain system.
// Actors are stateless decision-makers attached to dotions.
// Each tick, a dotion's brain runs its decide() method,
// observes the dotion's state, and can send messages back.

use crate::vm::value::Value;
use crate::dop::types::ActorTypeDef;
use std::collections::HashMap;

/// An actor registry — maps actor type names to their definitions.
pub struct ActorRegistry {
    types: HashMap<String, ActorTypeDef>,
}

impl ActorRegistry {
    pub fn new() -> Self { Self { types: HashMap::new() } }

    pub fn register(&mut self, name: String, def: ActorTypeDef) {
        self.types.insert(name, def);
    }

    pub fn get(&self, name: &str) -> Option<&ActorTypeDef> {
        self.types.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }
}

/// Describes what an actor brain decided to do this tick.
/// The VM executes these decisions after the brain runs.
#[derive(Debug, Clone)]
pub enum BrainDecision {
    /// Send a message to self
    SendSelf { msg: String, arg: Value },
    /// Send a message to another dotion by ID
    SendTo { target_id: u64, msg: String, arg: Value },
    /// Broadcast to all dotions
    Broadcast { msg: String, arg: Value },
    /// No decision this tick
    Idle,
}

