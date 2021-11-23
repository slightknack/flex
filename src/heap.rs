use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pointer(u64);

// Needs to do a few simple things:
// Returns all ranges greater than or equal to a given size
// when a range is added, merges neighboring ranges together
// when a range is removed, splits neighboring ranges

// TODO: add support for extending/shrinking the size of the range map
// TODO: verify behavior of range map and remove asserts and checked unwraps
// TODO: integrate the range set with the heap to combat fragmentation.

// keeps track of unallocated ranges of slots
pub struct RangeSet {
    // slots before, length of range
    ranges: BTreeMap<Pointer, usize>,
    // length -> start of range
    // if an entry size is present in the map, the pointer set must be non-empty.
    free: BTreeMap<usize, BTreeSet<Pointer>>,
}

impl RangeSet {
    pub fn mark_first(&mut self, slots: usize) -> Pointer {
        for (size, potential) in self.free.range(slots..) {
            todo!()
        }
        todo!()
    }

    pub fn mark_smaller(&mut self, pointer: Pointer, slots: usize) {
        // grab the full allocation
        let size = self.mark(pointer);
        assert!(slots < size);
        // let go of the end; may cause minor fragmentation
        self.free(Pointer(pointer.0 + slots as u64), size - slots);
    }

    pub fn mark(&mut self, pointer: Pointer) -> usize {
        // remove it from the ranges set, getting the size of the pointer
        let size = self.ranges.remove(&pointer).unwrap();
        // remove it from the free set, by inverse looking up by size
        assert!(self.free.get_mut(&size).unwrap().remove(&pointer));
        // if that was the last item, remove the entry from the map.
        if self.free.get(&size).unwrap().is_empty() {
            self.free.remove_entry(&size);
        }
        // return the size of the marked pointer
        return size;
    }

    pub fn free(&mut self, pointer: Pointer, slots: usize) {
        // merge it with any other nearby ranges
        // start with the range before
        if let Some((pointer_before, size)) = self.ranges.range(..pointer).rev().next() {
            // if the free ranges are back-to-back, we merge them by extending the old range
            if Pointer(pointer_before.0 + *size as u64) == pointer {
                // remove the pointer from its old size in the free map
                assert!(self.free.get_mut(size).unwrap().remove(pointer_before));
                // remove the pointer from the ranges map
                self.ranges.remove(pointer_before).unwrap();

                // use the new combined pointer
                pointer = *pointer_before;
                slots = size + slots;
            }
        }

        // then do the range after
        // pointer has not yet beed added to the ranges map,
        // so `pointer..` is technically exclusive
        if let Some((pointer_after, size)) = self.ranges.range(pointer..).next() {
            if Pointer(pointer.0 + slots as u64) == *pointer_after {
                // remove the pointer_after from its old size in the free map
                self.free.get_mut(size).unwrap().remove(pointer_after);
                // remove the pointer_after from the ranges map
                self.ranges.remove(pointer_after).unwrap();
                // extend the pointer to be longer
                slots += size;
            }
        }

        // add the pointer with its new size in the free map
        self.free.get_mut(&slots).unwrap().insert(pointer);
        // add the pointer with its new size to the ranges map
        assert!(self.ranges.insert(pointer, slots).is_none());
    }
}

pub struct Heap {
    data: Vec<u64>,
    free: RangeSet,
}

impl Heap {
    fn is_free(&self, pointer: Pointer, slots: usize) -> bool {
        todo!()
    }

    pub fn alloc(&mut self, slots: usize) -> Pointer {

    }

    pub fn realloc(&mut self, pointer: Pointer, old: usize, new: usize) -> Pointer {
        if new > old {
            // try allocation continiously
            if self.is_free(Pointer(pointer.0 + old as u64), new - old) {
                // increase the size of the current allocation
                todo!();
                return pointer;
            }

            // TODO: free before reallocation might open up space before?
            // reallocate
            let new_pointer = self.alloc(new);
            for slot in 0..old {
                self.data[new_pointer.0 + slot] = self.data[old_pointer.0 + slot];
            }
            // and free
            self.free(pointer, old);
            return new_pointer;
        } else if old > new {
            // free back half of allocation
            self.free(Pointer(pointer.0 + new), old - new);
            return pointer;
        }
    }

    pub fn read(&self, pointer: Pointer, slots: usize) -> &[u64] {

    }

    pub fn write(&mut self, pointer: Pointer, item: &mut [u64]) -> Pointer {

    }

    pub fn free(&mut self, pointer: Pointer, slots: usize) {

    }
}
