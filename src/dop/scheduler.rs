#![allow(dead_code)]
// FLUXIS — dop/scheduler.rs
// The DOP Scheduler: drives the tick cycle.
// The VM executes instructions. The Scheduler decides WHEN things happen.
//
// Each tick:
//   1. Sort dotions by tick_priority
//   2. Process mailboxes (run on-handlers)
//   3. Run actor brains (decide())
//   4. Run user tick{} block
//   5. Advance tick counter

use crate::vm::value::Value;
use crate::dop::types::{TickEngine, TickPhase};
use crate::error::FluxisError;

/// The Scheduler is the DOP runtime engine.
/// It is called by the tree-walking VM during tick() execution.
/// It never calls into the VM directly — instead it returns instructions
/// (via the VM's execute/call_function interface) to run handlers and brains.
pub struct Scheduler {
    pub engine: TickEngine,
    pub current_phase: TickPhase,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            engine: TickEngine::new(),
            current_phase: TickPhase::Advance,
        }
    }

    pub fn set_tick_block(&mut self, body: Vec<crate::ast::Statement>) {
        self.engine.set_block(body);
    }

    pub fn tick_count(&self) -> u64 { self.engine.tick_count }

    /// Advance one tick. The caller (VM) must supply:
    /// - A list of (var_name, dotion) pairs sorted by tick_priority
    /// - Callbacks to process mailboxes and run actor brains
    ///
    /// Returns the list of (var_name, Value) pairs with updated dotion state.
    pub fn run_tick<F1, F2, F3>(
        &mut self,
        dotions: Vec<(String, Value)>,
        mut process_mailbox: F1,
        mut run_brain: F2,
        mut run_tick_block: F3,
    ) -> Result<Vec<(String, Value)>, FluxisError>
    where
        F1: FnMut(String, Value) -> Result<Value, FluxisError>,
        F2: FnMut(String, Value) -> Result<Value, FluxisError>,
        F3: FnMut() -> Result<(), FluxisError>,
    {
        let mut state = dotions;

        // ── Phase 1: Mailbox processing ───────────────────────────────
        self.current_phase = TickPhase::MailboxProcessing;
        let mut updated = Vec::new();
        for (name, dotion) in state {
            let result = process_mailbox(name.clone(), dotion)?;
            updated.push((name, result));
        }
        state = updated;

        // ── Phase 2: Actor brain decisions ────────────────────────────
        self.current_phase = TickPhase::ActorDecision;
        let mut post_brain = Vec::new();
        for (name, dotion) in state {
            let has_brain = matches!(&dotion, Value::Dotion { brain: Some(_), .. });
            let result = if has_brain { run_brain(name.clone(), dotion)? } else { dotion };
            post_brain.push((name, result));
        }
        state = post_brain;

        // ── Phase 3: Tick block ───────────────────────────────────────
        self.current_phase = TickPhase::TickBlock;
        run_tick_block()?;

        // ── Phase 4: Advance ──────────────────────────────────────────
        self.current_phase = TickPhase::Advance;
        self.engine.advance();

        Ok(state)
    }

    /// Sort dotions by tick_priority (lower = runs first).
    pub fn sort_by_priority(dotions: &mut Vec<(String, Value)>) {
        dotions.sort_by_key(|(_, v)| match v {
            Value::Dotion { tick_priority, .. } => *tick_priority,
            _ => 0,
        });
    }
}

