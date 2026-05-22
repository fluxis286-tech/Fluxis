#![allow(dead_code)]
// FLUXIS — dop/types.rs
// Type definitions for the Dotion-Oriented Programming system.
// Pure data — no execution logic here.

use std::sync::atomic::{AtomicU64, Ordering};
use crate::ast::{Handler, DotionMethod, Statement, Expr};

// ── UNIQUE ID ─────────────────────────────────────────────────────────────
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a globally unique dotion instance ID.
pub fn new_id() -> u64 { NEXT_ID.fetch_add(1, Ordering::Relaxed) }

// ── DOTION TYPE DEFINITION ────────────────────────────────────────────────
/// A named dotion type (like a class blueprint).
/// Instances are created with `d = DotionName {}`.
#[derive(Clone, Debug)]
pub struct DotionTypeDef {
    /// Default field values (evaluated at instantiation time)
    pub fields:        Vec<(String, Expr)>,
    /// Structured methods (called with d.method())
    pub methods:       Vec<DotionMethod>,
    /// Message handlers (triggered by send/broadcast)
    pub handlers:      Vec<Handler>,
    /// Optional actor brain attached to this type
    pub brain:         Option<String>,
    /// Parent dotion type to inherit from
    pub extends:       Option<String>,
    /// Tags for broadcast_to filtering
    pub tags:          Vec<String>,
    /// Lower priority = runs first each tick (default 0)
    pub tick_priority: i64,
}

// ── ACTOR TYPE DEFINITION ─────────────────────────────────────────────────
/// An Actor is a stateless decision-maker attached to a dotion as its brain.
/// Separation of concerns:
///   Dotion = body  (what the entity IS  — state + reactions)
///   Actor  = brain (what the entity DOES — decisions each tick)
#[derive(Clone, Debug)]
pub struct ActorTypeDef {
    pub methods: Vec<DotionMethod>,
}

// ── TICK ENGINE ───────────────────────────────────────────────────────────
/// Owns the simulation clock. The DOP Scheduler drives it.
pub struct TickEngine {
    pub tick_count: u64,
    pub tick_block: Option<Vec<Statement>>,
}

impl TickEngine {
    pub fn new() -> Self { Self { tick_count: 0, tick_block: None } }
    pub fn set_block(&mut self, body: Vec<Statement>) { self.tick_block = Some(body); }
    pub fn advance(&mut self) { self.tick_count += 1; }
}

// ── EXECUTION PHASES ──────────────────────────────────────────────────────
/// The four phases of each tick cycle.
/// The Scheduler drives these in order.
#[derive(Debug, Clone, PartialEq)]
pub enum TickPhase {
    /// Phase 1: deliver all queued messages to their handlers
    MailboxProcessing,
    /// Phase 2: each actor brain's decide() method runs
    ActorDecision,
    /// Phase 3: the user-defined tick{} block runs
    TickBlock,
    /// Phase 4: tick counter increments, engine is ready for next tick
    Advance,
}

// ── DOTION IDENTITY ───────────────────────────────────────────────────────
/// Lightweight identity token for a dotion instance.
/// Used by the scheduler to track dotions without cloning their full state.
#[derive(Clone, Debug)]
pub struct DotionId {
    pub id:   u64,
    pub name: String,
}

impl DotionId {
    pub fn new(name: &str) -> Self { Self { id: new_id(), name: name.to_string() } }
}

