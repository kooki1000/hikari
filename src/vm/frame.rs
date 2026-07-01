use std::rc::Rc;

use crate::compiler::{Chunk, Instruction, Value};
use crate::lexer::Span;

// ── Call frame ────────────────────────────────────────────────────────────────

pub(super) struct Frame {
    // Shared with the owning Chunk (an O(1) refcount bump, not a deep copy of
    // the body). Frame 0 in the REPL rebuilds this slice when a line is added.
    pub(super) instructions: Rc<[Instruction]>,
    pub(super) ip: usize,
    pub(super) locals: Vec<Option<Value>>,
    // Span checkpoints for `instructions` (see Chunk::spans). Used to attach a
    // source location to a runtime error raised in this frame.
    pub(super) spans: Rc<[(usize, Span)]>,
    // The chunk's declared function name, if any (see Chunk::name). Used to
    // render a per-frame name in an uncaught error's stack trace.
    pub(super) name: Option<Rc<str>>,
}

// Initial local-slot capacity for a fresh frame. Slots beyond this are
// allocated on demand by set_local, so this is only a starting size that
// avoids reallocation for the common case, not a hard ceiling.
pub(super) const INITIAL_LOCALS: usize = 256;

impl Frame {
    pub(super) fn new(chunk: &Chunk, args: Vec<Value>) -> Self {
        let mut locals: Vec<Option<Value>> = vec![None; INITIAL_LOCALS.max(args.len())];
        // Seed parameter slots from args (left-to-right = slot 0, 1, …).
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = Some(arg);
        }
        Self {
            instructions: chunk.instructions.clone(),
            ip: 0,
            locals,
            spans: chunk.spans.clone(),
            name: chunk.name.clone(),
        }
    }

    /// The source span of the instruction at `ip` in this frame, if known.
    pub(super) fn span_at(&self, ip: usize) -> Option<Span> {
        self.spans
            .iter()
            .rev()
            .find(|(start, _)| *start <= ip)
            .map(|(_, span)| *span)
    }

    // Store into a local slot, growing the slot vector if the compiler
    // assigned a slot index beyond the current capacity. Slot indices are
    // allocated monotonically per function, so a large body can legitimately
    // exceed INITIAL_LOCALS; without this growth such a program would panic.
    pub(super) fn set_local(&mut self, slot: u16, val: Value) {
        let idx = slot as usize;
        if idx >= self.locals.len() {
            self.locals.resize(idx + 1, None);
        }
        self.locals[idx] = Some(val);
    }

    // Read a local slot. An out-of-range or never-written slot reads as None,
    // which callers surface as UninitializedLocal rather than panicking.
    pub(super) fn get_local(&self, slot: u16) -> Option<Value> {
        self.locals.get(slot as usize).cloned().flatten()
    }
}

// ── Try/catch handler ────────────────────────────────────────────────────────

pub(super) struct TryHandler {
    pub(super) catch_target: usize,
    pub(super) error_slot: u16,
    pub(super) stack_len: usize,
    pub(super) frame_depth: usize,
}
