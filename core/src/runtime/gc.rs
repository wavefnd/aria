use std::collections::HashSet;
use crate::runtime::heap::{Heap, HeapValue, ObjectRef};
use crate::runtime::stack::Stack;
use crate::runtime::frame::Frame;

pub struct Gc {
    pub debug_mode: bool,
}

impl Gc {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    pub fn collect(&self, heap: &mut Heap, stack: &Stack) {
        if self.debug_mode {
            println!("Starting GC (Mark-Sweep) ...");
        }

        let marked = self.mark_from_stack(heap, stack);

        let before = heap.object_count();
        heap.retain_alive(&marked);
        let after = heap.object_count();

        if self.debug_mode {
            println!(
                "GC finished: before={} after={} collected={}",
                before,
                after,
                before - after
            );
        }
    }

    fn mark_from_stack(&self, heap: &Heap, stack: &Stack) -> HashSet<u64> {
        let mut marked: HashSet<u64> = HashSet::new();

        for frame in stack.iter_frames() {
            for val in &frame.local_vars {
                self.mark_value(heap, val, &mut marked);
            }
            for val in &frame.operand_stack {
                self.mark_value(heap, val, &mut marked);
            }
        }

        if self.debug_mode {
            println!("🪄 Marked {} reachable objects", marked.len());
        }

        marked
    }

    fn mark_value(&self, heap: &Heap, value: &HeapValue, marked: &mut HashSet<u64>) {
        if let HeapValue::Object(obj_ref) = value {
            self.mark_object_recursive(heap, obj_ref, marked);
        }
    }

    fn mark_object_recursive(&self, heap: &Heap, obj_ref: &ObjectRef, marked: &mut HashSet<u64>) {
        if marked.contains(&obj_ref.id) {
            return;
        }

        marked.insert(obj_ref.id);

        if let Some(obj) = heap.get(obj_ref.id) {
            for field_value in obj.fields.values() {
                self.mark_value(heap, field_value, marked);
            }
        }
    }
}


impl Heap {
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn retain_alive(&mut self, marked: &HashSet<u64>) {
        self.objects.retain(|id, _| marked.contains(id));
    }

    pub fn iter_objects(&self) -> impl Iterator<Item = (&u64, &ObjectRef)> {
        self.objects.iter()
    }
}

impl Stack {
    pub fn iter_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter()
    }
}