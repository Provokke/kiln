//! Generational garbage-collected heap.
//!
//! * **Nursery**: a bump-allocated arena (objects are appended to a vector).
//!   A *minor GC* evacuates (copies) every nursery object reachable from the
//!   roots or the remembered set into the old generation, leaving forwarding
//!   pointers behind, then resets the nursery in O(1). Surviving objects are
//!   promoted immediately (no aging).
//! * **Old generation**: a slot arena with a free list, collected by
//!   **mark-sweep** during a *major GC*.
//! * **Write barrier**: every mutation of an old object goes through
//!   [`Heap::get_mut`], which records the object in the remembered set so the
//!   next minor GC treats its fields as roots.
//!
//! Collections only happen at VM safepoints (between bytecode instructions),
//! where every live reference is reachable from the precise root set the VM
//! enumerates. In *stress mode* every allocation requests a collection at
//! the next safepoint.

use crate::bytecode::FuncProto;
use crate::value::{Str, Value};
use std::collections::HashMap;
use std::rc::Rc;

const NURSERY_BIT: u32 = 1 << 31;

/// A reference to a heap object. The top bit selects the generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GcRef(pub u32);

impl GcRef {
    pub fn is_nursery(self) -> bool {
        self.0 & NURSERY_BIT != 0
    }
    fn index(self) -> usize {
        (self.0 & !NURSERY_BIT) as usize
    }
}

pub type ShapeId = u32;

/// Callback that enumerates every root reference, passing each to the given visitor.
pub type RootVisitor<'a> = &'a mut dyn FnMut(&mut dyn FnMut(&mut GcRef));

/// A hidden class: an ordered list of property names mapping to slot indices,
/// plus a transition table to child shapes.
#[derive(Default)]
pub struct Shape {
    pub keys: Vec<Str>,
    map: HashMap<Str, u32>,
    transitions: HashMap<Str, ShapeId>,
}

pub type NativeFn = fn(&mut crate::vm::Vm, Value, &[Value]) -> Result<Value, Value>;

pub enum ObjKind {
    Plain,
    Array(Vec<Value>),
    Closure {
        func: Rc<FuncProto>,
        cells: Vec<GcRef>,
    },
    Native {
        f: NativeFn,
        name: &'static str,
    },
    /// A boxed variable captured by a closure.
    Cell(Value),
}

pub struct Obj {
    pub kind: ObjKind,
    pub shape: ShapeId,
    pub slots: Vec<Value>,
    pub proto: Option<GcRef>,
    marked: bool,
    remembered: bool,
}

impl Obj {
    pub fn new(kind: ObjKind, proto: Option<GcRef>) -> Obj {
        Obj {
            kind,
            shape: 0,
            slots: Vec::new(),
            proto,
            marked: false,
            remembered: false,
        }
    }

    /// Visit every reference held by this object.
    fn trace(&mut self, f: &mut dyn FnMut(&mut GcRef)) {
        for v in &mut self.slots {
            if let Value::Obj(r) = v {
                f(r)
            }
        }
        if let Some(p) = &mut self.proto {
            f(p)
        }
        match &mut self.kind {
            ObjKind::Array(items) => {
                for v in items {
                    if let Value::Obj(r) = v {
                        f(r)
                    }
                }
            }
            ObjKind::Closure { cells, .. } => {
                for c in cells {
                    f(c)
                }
            }
            ObjKind::Cell(Value::Obj(r)) => f(r),
            _ => {}
        }
    }
}

enum NurserySlot {
    Live(Obj),
    Forwarded(GcRef),
}

#[derive(Default, Debug, Clone)]
pub struct GcStats {
    pub minor_gcs: u64,
    pub major_gcs: u64,
    pub promoted: u64,
    pub swept: u64,
    pub allocated: u64,
}

pub struct Heap {
    nursery: Vec<NurserySlot>,
    old: Vec<Option<Obj>>,
    free: Vec<u32>,
    old_live: usize,
    remembered: Vec<GcRef>,
    scan_queue: Vec<GcRef>,
    pub shapes: Vec<Shape>,
    pub nursery_limit: usize,
    pub major_threshold: usize,
    pub stress: bool,
    pub gc_pending: bool,
    pub stats: GcStats,
}

impl Default for Heap {
    fn default() -> Heap {
        Heap {
            nursery: Vec::new(),
            old: Vec::new(),
            free: Vec::new(),
            old_live: 0,
            remembered: Vec::new(),
            scan_queue: Vec::new(),
            shapes: vec![Shape::default()],
            nursery_limit: 4096,
            major_threshold: 50_000,
            stress: false,
            gc_pending: false,
            stats: GcStats::default(),
        }
    }
}

impl Heap {
    pub fn alloc(&mut self, obj: Obj) -> GcRef {
        self.stats.allocated += 1;
        let idx = self.nursery.len() as u32;
        self.nursery.push(NurserySlot::Live(obj));
        if self.stress || self.nursery.len() >= self.nursery_limit {
            self.gc_pending = true;
        }
        GcRef(idx | NURSERY_BIT)
    }

    pub fn get(&self, r: GcRef) -> &Obj {
        if r.is_nursery() {
            match &self.nursery[r.index()] {
                NurserySlot::Live(o) => o,
                NurserySlot::Forwarded(_) => panic!("stale nursery reference {r:?}"),
            }
        } else {
            self.old[r.index()].as_ref().expect("dangling old-gen reference")
        }
    }

    /// Mutable access with a write barrier: old objects are added to the
    /// remembered set (conservatively, on any mutation).
    pub fn get_mut(&mut self, r: GcRef) -> &mut Obj {
        if r.is_nursery() {
            match &mut self.nursery[r.index()] {
                NurserySlot::Live(o) => o,
                NurserySlot::Forwarded(_) => panic!("stale nursery reference {r:?}"),
            }
        } else {
            let o = self.old[r.index()].as_mut().expect("dangling old-gen reference");
            if !o.remembered {
                o.remembered = true;
                self.remembered.push(r);
            }
            o
        }
    }

    pub fn nursery_len(&self) -> usize {
        self.nursery.len()
    }
    pub fn old_live(&self) -> usize {
        self.old_live
    }
    pub fn remembered_len(&self) -> usize {
        self.remembered.len()
    }

    // ------------------------------------------------------------ shapes

    pub fn shape_lookup(&self, shape: ShapeId, key: &str) -> Option<u32> {
        self.shapes[shape as usize].map.get(key).copied()
    }

    pub fn shape_add(&mut self, shape: ShapeId, key: &Str) -> ShapeId {
        if let Some(s) = self.shapes[shape as usize].transitions.get(key) {
            return *s;
        }
        let parent = &self.shapes[shape as usize];
        let mut keys = parent.keys.clone();
        let mut map = parent.map.clone();
        map.insert(key.clone(), keys.len() as u32);
        keys.push(key.clone());
        let id = self.shapes.len() as ShapeId;
        self.shapes.push(Shape {
            keys,
            map,
            transitions: HashMap::new(),
        });
        self.shapes[shape as usize].transitions.insert(key.clone(), id);
        id
    }

    // ------------------------------------------------------------ collection

    fn alloc_old(&mut self, obj: Obj) -> GcRef {
        self.old_live += 1;
        if let Some(i) = self.free.pop() {
            self.old[i as usize] = Some(obj);
            GcRef(i)
        } else {
            self.old.push(Some(obj));
            GcRef(self.old.len() as u32 - 1)
        }
    }

    fn evacuate(&mut self, r: &mut GcRef) {
        if !r.is_nursery() {
            return;
        }
        let i = r.index();
        if let NurserySlot::Forwarded(to) = self.nursery[i] {
            *r = to;
            return;
        }
        let NurserySlot::Live(obj) = std::mem::replace(&mut self.nursery[i], NurserySlot::Forwarded(GcRef(0))) else {
            unreachable!()
        };
        let to = self.alloc_old(obj);
        self.nursery[i] = NurserySlot::Forwarded(to);
        self.stats.promoted += 1;
        self.scan_queue.push(to);
        *r = to;
    }

    /// Scan an old object's fields, evacuating nursery referents.
    fn scan_old(&mut self, r: GcRef) {
        let Some(mut obj) = self.old[r.index()].take() else { return };
        obj.trace(&mut |c| self.evacuate(c));
        obj.remembered = false;
        self.old[r.index()] = Some(obj);
    }

    /// Copying minor collection. `roots` must visit every root reference.
    pub fn minor_gc(&mut self, roots: RootVisitor<'_>) {
        self.stats.minor_gcs += 1;
        roots(&mut |r| self.evacuate(r));
        let remembered = std::mem::take(&mut self.remembered);
        for r in remembered {
            self.scan_old(r);
        }
        while let Some(r) = self.scan_queue.pop() {
            self.scan_old(r);
        }
        self.nursery.clear();
        self.gc_pending = false;
    }

    /// Full collection: a minor GC to empty the nursery, then mark-sweep of the old generation.
    pub fn major_gc(&mut self, roots: RootVisitor<'_>) {
        self.minor_gc(roots);
        self.stats.major_gcs += 1;
        let mut stack: Vec<GcRef> = Vec::new();
        roots(&mut |r| stack.push(*r));
        while let Some(r) = stack.pop() {
            let obj = self.old[r.index()].as_mut().expect("dangling ref during mark");
            if obj.marked {
                continue;
            }
            obj.marked = true;
            obj.trace(&mut |c| stack.push(*c));
        }
        for (i, slot) in self.old.iter_mut().enumerate() {
            if let Some(o) = slot {
                if o.marked {
                    o.marked = false;
                } else {
                    *slot = None;
                    self.free.push(i as u32);
                    self.old_live -= 1;
                    self.stats.swept += 1;
                }
            }
        }
        // Remembered entries may point to swept objects; they are all clean after a minor GC anyway.
        self.remembered.clear();
        for o in self.old.iter_mut().flatten() {
            o.remembered = false;
        }
        self.major_threshold = (self.old_live * 2).max(50_000);
    }

    pub fn wants_major(&self) -> bool {
        self.old_live >= self.major_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arr(h: &mut Heap, items: Vec<Value>) -> GcRef {
        h.alloc(Obj::new(ObjKind::Array(items), None))
    }

    fn items(h: &Heap, r: GcRef) -> &Vec<Value> {
        match &h.get(r).kind {
            ObjKind::Array(v) => v,
            _ => panic!(),
        }
    }

    #[test]
    fn minor_gc_copies_reachable_and_forwards() {
        let mut h = Heap::default();
        let leaf = arr(&mut h, vec![Value::Num(42.0)]);
        let _garbage = arr(&mut h, vec![]);
        let mut root = arr(&mut h, vec![Value::Obj(leaf), Value::Obj(leaf)]);
        h.minor_gc(&mut |v| v(&mut root));
        assert!(!root.is_nursery());
        assert_eq!(h.nursery_len(), 0);
        assert_eq!(h.old_live(), 2, "only root and leaf survive");
        let it = items(&h, root).clone();
        // both references were forwarded to the same copy
        assert!(matches!((&it[0], &it[1]), (Value::Obj(a), Value::Obj(b)) if a == b && !a.is_nursery()));
        let Value::Obj(l) = it[0] else { panic!() };
        assert!(matches!(items(&h, l)[0], Value::Num(n) if n == 42.0));
    }

    #[test]
    fn write_barrier_keeps_young_object_alive() {
        let mut h = Heap::default();
        let mut root = arr(&mut h, vec![]);
        h.minor_gc(&mut |v| v(&mut root)); // root is now old
        let young = arr(&mut h, vec![Value::Num(7.0)]);
        if let ObjKind::Array(v) = &mut h.get_mut(root).kind {
            v.push(Value::Obj(young));
        }
        assert_eq!(h.remembered_len(), 1);
        h.minor_gc(&mut |v| v(&mut root));
        let Value::Obj(y) = items(&h, root)[0] else { panic!() };
        assert!(!y.is_nursery());
        assert!(matches!(items(&h, y)[0], Value::Num(n) if n == 7.0));
    }

    #[test]
    fn major_gc_sweeps_unreachable_cycles() {
        let mut h = Heap::default();
        let a = arr(&mut h, vec![]);
        let b = arr(&mut h, vec![Value::Obj(a)]);
        if let ObjKind::Array(v) = &mut h.get_mut(a).kind {
            v.push(Value::Obj(b));
        }
        let mut keep = arr(&mut h, vec![Value::Obj(a)]);
        h.minor_gc(&mut |v| v(&mut keep));
        assert_eq!(h.old_live(), 3);
        // drop the reference to the cycle
        if let ObjKind::Array(v) = &mut h.get_mut(keep).kind {
            v.clear();
        }
        h.major_gc(&mut |v| v(&mut keep));
        assert_eq!(h.old_live(), 1);
        assert_eq!(h.stats.swept, 2);
        // freed slots are reused
        let mut n = arr(&mut h, vec![]);
        h.minor_gc(&mut |v| {
            v(&mut keep);
            v(&mut n)
        });
        assert!(n.0 < 3);
    }

    #[test]
    fn shapes_share_transitions() {
        let mut h = Heap::default();
        let x: Str = "x".into();
        let y: Str = "y".into();
        let s1 = h.shape_add(0, &x);
        let s2 = h.shape_add(s1, &y);
        assert_eq!(h.shape_add(0, &x), s1);
        assert_eq!(h.shape_add(s1, &y), s2);
        assert_eq!(h.shape_lookup(s2, "y"), Some(1));
        assert_eq!(h.shape_lookup(s1, "y"), None);
        let s3 = h.shape_add(0, &y);
        assert_ne!(s3, s2);
    }
}
