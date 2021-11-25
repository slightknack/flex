use std::collections::{BTreeMap, BTreeSet};

/// A tagged pointer to some data in a managed heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pointer(u64);

// Needs to do a few simple things:
// Returns all ranges greater than or equal to a given size
// when a range is added, merges neighboring ranges together
// when a range is removed, splits neighboring ranges

/// Keeps track of unallocated ranges of slots
/// When a pointer is freed, it's range is merged with other ranges
/// We use a pair of BTreeMaps to keep this snappy under the hood.
#[derive(Debug)]
pub struct RangeSet {
    capacity: usize,
    // slots before, length of range
    ranges: BTreeMap<Pointer, usize>,
    // length -> start of range
    // if an entry size is present in the map, the pointer set must be non-empty.
    free: BTreeMap<usize, BTreeSet<Pointer>>,
}

impl RangeSet {
    pub fn new() -> RangeSet {
        RangeSet {
            capacity: 0,
            ranges:   BTreeMap::new(),
            free:     BTreeMap::new(),
        }
    }

    /// Adds some capacity to the heap
    pub fn add_free_capacity(&mut self, slots: usize) {
        self.free(Pointer(self.capacity as u64), slots);
        self.capacity += slots;
    }

    pub fn new_with_free_capacity(slots: usize) -> RangeSet {
        let mut empty = RangeSet::new();
        empty.add_free_capacity(slots);
        empty
    }

    /// Returns a pointer and the size to increase the allocation by.
    /// The backing allocation size must be increased according to the returned size.
    /// Do not call `add_free_capacity` with the returned size of this method,
    /// because the allocation is used, not free.
    pub fn mark_first(&mut self, slots: usize) -> (Pointer, usize) {
        // try filling the smallest earliest gap possible.
        if let Some((size, potential)) = self.free.range(slots..).next() {
            let pointer = *potential.iter().next().unwrap();
            self.mark_smaller(pointer, slots);
            return (pointer, 0);
        }

        // if the last range is a tail range, try extending it
        if let Some((tail, size)) = self.ranges.iter().rev().next() {
            // copy to please the borrow checker gods
            let (tail, size) = (*tail, *size);
            // this free range goes right up to the end
            if tail.0 as usize + size == self.capacity {
                self.mark(tail);
                let remaining = slots - size;
                self.capacity += remaining;
                return (tail, remaining)
            }
        }

        let pointer = Pointer(self.capacity as u64);
        self.capacity += slots;
        return (pointer, slots);
    }

    /// Mark a pointer for use reserving a certain number of slots,
    /// returns the extra free space to the heap.
    pub fn mark_smaller(&mut self, pointer: Pointer, slots: usize) {
        // grab the full allocation
        let size = self.mark(pointer);
        if size == slots { return; }

        // let go of the end; may cause minor fragmentation
        assert!(slots < size);
        self.free(Pointer(pointer.0 + slots as u64), size - slots);
    }

    /// Mark a pointer for use, returns the size of the full allocation
    fn mark(&mut self, pointer: Pointer) -> usize {
        // remove it from the ranges set, getting the size of the pointer
        let size = self.ranges.remove(&pointer).unwrap();
        // remove it from the free set, by inverse looking up by size
        assert!(self.free.get_mut(&size).unwrap().remove(&pointer));
        // if the pointer was the last of a given size, remove the entry from the map
        if self.free.get(&size).unwrap().is_empty() {
            self.free.remove_entry(&size);
        }
        // return the size of the marked pointer
        return size;
    }

    /// Returns capacity that the heap can be shrunk by if freeing a tail allocation
    pub fn free(&mut self, mut pointer: Pointer, mut slots: usize) -> usize {
        // merge it with any other nearby ranges
        // start with the range before
        if let Some((pointer_before, size)) = self.ranges.range(..pointer).rev().next() {
            let (pointer_before, size) = (*pointer_before, *size);

            // if the free ranges are back-to-back, we merge them by extending the old range
            if Pointer(pointer_before.0 + size as u64) == pointer {
                // use the new combined pointer
                self.mark(pointer_before);
                pointer = pointer_before;
                slots = size + slots;
            }
        }

        // then do the range after
        // pointer has not yet beed added to the ranges map,
        // so `pointer..` is technically exclusive
        if let Some((pointer_after, size)) = self.ranges.range(pointer..).next() {
            let (pointer_after, size) = (*pointer_after, *size);
            if Pointer(pointer.0 + slots as u64) == pointer_after {
                // extend the pointer to be longer
                self.mark(pointer_after);
                slots += size;
            }
        }

        // if this is a tail free, reduce the size of the heap
        if pointer.0 as usize + slots == self.capacity {
            self.capacity -= slots;
            return slots;
        }

        // add the pointer with its new size in the free map
        // add the pointer with its new size to the ranges map
        if let Some(s) = self.free.get_mut(&slots) {
            s.insert(pointer);
        } else {
            let mut pointers = BTreeSet::new();
            pointers.insert(pointer);
            self.free.insert(slots, pointers);
        }

        // not a tail free, there still may be an allocation after this one
        // return 0 to keep slots
        assert!(self.ranges.insert(pointer, slots).is_none());
        return 0;
    }
}

#[derive(Debug)]
pub struct Heap {
    data: Vec<u64>,
    free: RangeSet,
}

impl Heap {
    /// Constructs new empty heap
    pub fn new() -> Heap {
        Heap {
            data: vec![],
            free: RangeSet::new(),
        }
    }

    pub fn draw_free(&self) {
        // print!("|");
        let mut old = 0;
        let mut unused = 0;
        for (key, value) in self.free.ranges.iter() {
            // print!("{}", "_".repeat(key.0 as usize - old));
            // print!("{}", "X".repeat(*value));
            unused += value;
            // old = key.0 as usize;
        }
        // print!("{}", "_".repeat(self.free.capacity - old));
        // println!("|");
        println!("==== INFO ====");
        println!("heap size:       {} bytes", self.data.len() * 8);
        println!("total slots:     {} slots", self.data.len());
        println!("disjoint ranges: {} slots", self.free.ranges.len());
        let pct = (unused as f64 / self.free.capacity as f64) * 100.0;
        println!("fragmentation:   {} / {} = {:.2}%", unused, self.free.capacity, pct);
    }

    /// Allocate a pointer of a given size.
    /// Returns the smallest first allocation that will fit the pointer.
    pub fn alloc(&mut self, slots: usize) -> Pointer {
        let (pointer, extra_capacity) = self.free.mark_first(slots);

        // increase the size of the allocation if needed.
        self.data.extend((0..extra_capacity).map(|_| 0));
        return pointer;
    }

    // TODO: tail allocations.
    /// Returns whether a pointer of a given size is free at a given point.
    /// Used to determine whether reallocation in place is possible.
    fn is_free(&self, pointer: Pointer, slots: usize) -> bool {
        // get the first pointer before or at the one specified.
        if let Some((p, free_range)) = self.free.ranges.range(..=pointer).rev().next() {
            // check that the free range covers the range of the pointer in question
            let p_end = p.0 as usize + free_range;
            let pointer_end = pointer.0 as usize + slots;

            // for a pointer to be free it must be in the range!
            if p_end >= pointer_end {
                return true;
            }

            // TODO: tail allocations; need to increase the allocation size.
            // || self.free.capacity == p_end
            //     && self.free.capacity < pointer_end
        }

        false
    }

    /// Reallocates an allocation to a larger size
    /// Tries to reallocate in place, but moves the allocation if needed.
    pub fn realloc(&mut self, pointer: Pointer, old: usize, new: usize) -> Pointer {
        if new > old {
            // try allocation continiously
            let tail = Pointer(pointer.0 + old as u64);
            if self.is_free(tail, new - old) {
                // increase the size of the current allocation
                self.free.mark_smaller(tail, new - old);
                return pointer;
            }

            // TODO: free before reallocation might open up space before?
            // reallocate new larger allocation, copy over data.
            let new_pointer = self.alloc(new);
            for slot in 0..old {
                self.data[new_pointer.0 as usize + slot] = self.data[pointer.0 as usize + slot];
            }
            // and free old small allocation
            self.free(pointer, old);
            return new_pointer;
        } else if old > new {
            // free back half of allocation
            self.free(Pointer(pointer.0 + new as u64), old - new);
        }

        // they're equal, so do nothing
        return pointer;
    }

    // Reads a single slot relative to a pointer.
    pub fn read_slot(&self, pointer: Pointer, slot: usize) -> u64 {
        self.data[pointer.0 as usize + slot]
    }

    // Reads a range of data.
    pub fn read(&self, pointer: Pointer, slots: usize) -> &[u64] {
        let start = pointer.0 as usize;
        &self.data[start..(start + slots)]
    }

    pub fn write(&mut self, pointer: Pointer, item: &mut [u64]) -> Pointer {
        // todo: pointer tagging, change `.0` to `.idx()` as add a method `.is_owned()`
        todo!("Copy on write");
    }

    pub fn free(&mut self, pointer: Pointer, slots: usize) {
        let unneeded_capacity = self.free.free(pointer, slots);
        self.data.truncate(self.data.len() - unneeded_capacity);
    }
}


#[cfg(test)]
pub mod tests {
    use super::*;

    fn random_alloc_size(rng: &mut attorand::Rng) -> usize {
         rng.next_byte() as usize + 1
    }

    #[test]
    pub fn stress_test_heap() {
        let mut heap = Heap::new();
        let mut pointers = BTreeMap::new();
        let mut rng = attorand::Rng::new_default();

        for i in 0..100000 {
            let size = random_alloc_size(&mut rng);
            let pointer = heap.alloc(size);
            pointers.insert(i, (pointer, size));

            let index = rng.next_u64_max((pointers.len() - 1) as u64) as usize;
            if rng.next_bool() {
                let (index, (to_modify, old_size)) = pointers.iter().nth(index).unwrap();
                let index = *index;

                if rng.next_bool() {
                    let new_size = random_alloc_size(&mut rng);
                    let pointer = heap.realloc(*to_modify, *old_size, new_size);
                    pointers.insert(index, (pointer, new_size));
                } else {
                    heap.free(*to_modify, *old_size);
                    pointers.remove(&index);
                }
            }
        }

        heap.draw_free();
    }

    #[test]
    pub fn stress_test_native() {
        let mut rng = attorand::Rng::new_default();
        let mut pointers = BTreeMap::new();

        for i in 0..100000 {
            let size = random_alloc_size(&mut rng);
            let pointer = vec![0; size];
            pointers.insert(i, (pointer, size));

            let index = rng.next_u64_max((pointers.len() - 1) as u64) as usize;
            if rng.next_bool() {
                let (index, _) = pointers.iter().nth(index).unwrap();
                let index = *index;

                if rng.next_bool() {
                    let new_size = random_alloc_size(&mut rng);
                    let pointer = vec![0; new_size];
                    pointers.insert(index, (pointer, new_size));
                } else {
                    pointers.remove(&index);
                }
            }
        }
    }
}
