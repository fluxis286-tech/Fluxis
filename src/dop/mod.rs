// FLUXIS — dop/mod.rs
// Dotion-Oriented Programming subsystem.
//
// ┌─────────────────────────────────────────────────────────────────┐
// │  ARCHITECTURE                                                   │
// │                                                                 │
// │  VM  ──executes instructions──►  Core / tree-walking VM        │
// │  DOP ──decides when to run──►  Scheduler                       │
// │                                                                 │
// │  The VM never calls the Scheduler directly.                     │
// │  The VM's tick() statement hands control to the Scheduler,      │
// │  which then calls back into the VM to run handlers and brains.  │
// └─────────────────────────────────────────────────────────────────┘
//
//   types.rs     — DotionTypeDef, ActorTypeDef, TickEngine, TickPhase
//   scheduler.rs — Scheduler (drives the tick cycle)
//   actors.rs    — ActorRegistry, BrainDecision

pub mod types;
pub mod scheduler;
pub mod actors;

// Flat re-exports — allow unused since these are public API surface
#[allow(unused_imports)]
pub use types::{new_id, DotionTypeDef, ActorTypeDef, TickEngine, TickPhase, DotionId};
#[allow(unused_imports)]
pub use scheduler::Scheduler;
#[allow(unused_imports)]
pub use actors::{ActorRegistry, BrainDecision};

