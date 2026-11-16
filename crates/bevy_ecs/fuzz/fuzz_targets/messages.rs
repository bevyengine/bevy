#![no_main]
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use arbitrary::Arbitrary;
use bevy_ecs::message::{MessageCursor, MessageRegistry, Messages};
use bevy_ecs::prelude::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum MessageOp {
    WriteSmall(u32),
    WriteLarge(u64, u64),
    WriteBatch(Vec<u32>),

    Update,

    ReadCursor(u8),
    ReadAndVerifyOrder(u8),
    GetMessage(u8),

    Drain,
    ClearCursor(u8),

    CheckLen,

    TriggerEvent(u32),
    TriggerEntityEvent(u8, u32),
}

#[derive(Debug, Arbitrary)]
struct MessageFuzzInput {
    ops: Vec<MessageOp>,
}

#[derive(Message, Debug, Clone, PartialEq)]
struct SmallMsg(u32);

#[derive(Message, Debug, Clone, PartialEq)]
struct LargeMsg(u64, u64);

#[derive(Event, Debug, Clone)]
struct FuzzEvent(u32);

#[derive(EntityEvent, Debug, Clone)]
struct FuzzEntityEvent {
    entity: Entity,
    value: u32,
}

struct Shadow {
    written_small: Vec<u32>,
    written_ids: Vec<usize>,
    update_count: u32,
    update_boundaries: Vec<usize>,
    total_writes: usize,
    expected_event_count: u32,
    expected_entity_event_count: u32,
}

impl Shadow {
    fn new() -> Self {
        Self {
            written_small: Vec::new(),
            written_ids: Vec::new(),
            update_count: 0,
            update_boundaries: Vec::new(),
            total_writes: 0,
            expected_event_count: 0,
            expected_entity_event_count: 0,
        }
    }

    fn write_small(&mut self, v: u32) -> usize {
        let id = self.total_writes;
        self.total_writes += 1;
        self.written_small.push(v);
        self.written_ids.push(id);
        id
    }

    fn update(&mut self) {
        self.update_count += 1;
        self.update_boundaries.push(self.written_small.len());
    }

    fn drain(&mut self) {
        self.written_small.clear();
        self.written_ids.clear();
        self.update_boundaries.clear();
    }

    fn surviving_small_messages(&self) -> &[u32] {
        if self.update_boundaries.len() < 2 {
            &self.written_small
        } else {
            let cutoff = self.update_boundaries[self.update_boundaries.len() - 2];
            &self.written_small[cutoff..]
        }
    }
}

fuzz_target!(|input: MessageFuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    MessageRegistry::register_message::<SmallMsg>(&mut world);
    MessageRegistry::register_message::<LargeMsg>(&mut world);

    let mut shadow = Shadow::new();
    let mut alive: Vec<Entity> = Vec::new();

    let mut cursors: Vec<MessageCursor<SmallMsg>> = Vec::new();
    {
        let messages = world.resource::<Messages<SmallMsg>>();
        for _ in 0..4 {
            cursors.push(messages.get_cursor());
        }
    }

    let event_count = Arc::new(AtomicU32::new(0));
    let entity_event_count = Arc::new(AtomicU32::new(0));

    {
        let c = event_count.clone();
        world.add_observer(move |_: On<FuzzEvent>| {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }
    {
        let c = entity_event_count.clone();
        world.add_observer(move |_: On<FuzzEntityEvent>| {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }

    for op in &input.ops {
        match op {
            MessageOp::WriteSmall(v) => {
                let id = world.write_message(SmallMsg(*v));
                let expected_id = shadow.write_small(*v);
                if let Some(id) = id {
                    assert_eq!(
                        id.id, expected_id,
                        "Message ID mismatch: got {}, expected {}",
                        id.id, expected_id
                    );
                }
            }

            MessageOp::WriteLarge(a, b) => {
                world.write_message(LargeMsg(*a, *b));
            }

            MessageOp::WriteBatch(values) => {
                if values.len() <= 64 {
                    let msgs: Vec<SmallMsg> = values.iter().map(|v| SmallMsg(*v)).collect();
                    let messages = world.resource_mut::<Messages<SmallMsg>>();
                    let batch_ids = messages.into_inner().write_batch(msgs);
                    let count = batch_ids.count();
                    assert_eq!(count, values.len(), "Batch write count mismatch");
                    for v in values {
                        shadow.write_small(*v);
                    }
                }
            }

            MessageOp::Update => {
                shadow.update();
                world
                    .resource_mut::<Messages<SmallMsg>>()
                    .into_inner()
                    .update();
                world
                    .resource_mut::<Messages<LargeMsg>>()
                    .into_inner()
                    .update();
            }

            MessageOp::ReadCursor(cursor_idx) => {
                let ci = (*cursor_idx as usize) % cursors.len();
                let messages = world.resource::<Messages<SmallMsg>>();
                let read: Vec<SmallMsg> = cursors[ci].read(messages).cloned().collect();
                for msg in &read {
                    assert!(
                        shadow.written_small.contains(&msg.0),
                        "Read message {:?} that was never written",
                        msg
                    );
                }
            }

            MessageOp::ReadAndVerifyOrder(cursor_idx) => {
                let ci = (*cursor_idx as usize) % cursors.len();
                let messages = world.resource::<Messages<SmallMsg>>();
                let read_with_ids: Vec<_> = cursors[ci]
                    .read_with_id(messages)
                    .map(|(msg, id)| (msg.clone(), id.id))
                    .collect();
                for window in read_with_ids.windows(2) {
                    assert!(
                        window[0].1 < window[1].1,
                        "Message IDs not monotonic: {} >= {}",
                        window[0].1,
                        window[1].1
                    );
                }
            }

            MessageOp::GetMessage(id_idx) => {
                let messages = world.resource::<Messages<SmallMsg>>();
                let id = (*id_idx as usize) % (shadow.total_writes.max(1));
                if let Some((msg, msg_id)) = messages.get_message(id) {
                    assert_eq!(msg_id.id, id, "get_message returned wrong ID");
                    assert!(
                        shadow.written_small.contains(&msg.0),
                        "get_message returned unknown message {:?}",
                        msg
                    );
                }
            }

            MessageOp::Drain => {
                let messages = world.resource_mut::<Messages<SmallMsg>>();
                let drained: Vec<SmallMsg> = messages.into_inner().drain().collect();
                for msg in &drained {
                    assert!(
                        shadow.written_small.contains(&msg.0),
                        "Drained message {:?} that was never written",
                        msg
                    );
                }
                shadow.drain();
            }

            MessageOp::CheckLen => {
                let messages = world.resource::<Messages<SmallMsg>>();
                let surviving = shadow.surviving_small_messages();
                assert_eq!(
                    messages.len(),
                    surviving.len(),
                    "Messages len mismatch: actual={}, expected={}",
                    messages.len(),
                    surviving.len()
                );
                assert_eq!(
                    messages.is_empty(),
                    surviving.is_empty(),
                    "Messages is_empty mismatch"
                );
            }

            MessageOp::ClearCursor(cursor_idx) => {
                let ci = (*cursor_idx as usize) % cursors.len();
                let messages = world.resource::<Messages<SmallMsg>>();
                cursors[ci].clear(messages);
                let count = cursors[ci].len(messages);
                assert_eq!(count, 0, "Cursor not empty after clear");
            }

            MessageOp::TriggerEvent(v) => {
                shadow.expected_event_count += 1;
                world.trigger(FuzzEvent(*v));
            }

            MessageOp::TriggerEntityEvent(entity_idx, v) => {
                if alive.is_empty() {
                    alive.push(world.spawn_empty().id());
                }
                let e = alive[(*entity_idx as usize) % alive.len()];
                shadow.expected_entity_event_count += 1;
                world.trigger(FuzzEntityEvent {
                    entity: e,
                    value: *v,
                });
            }
        }
    }

    assert_eq!(
        event_count.load(Ordering::Relaxed),
        shadow.expected_event_count,
        "FuzzEvent trigger count mismatch"
    );
    assert_eq!(
        entity_event_count.load(Ordering::Relaxed),
        shadow.expected_entity_event_count,
        "FuzzEntityEvent trigger count mismatch"
    );
});
