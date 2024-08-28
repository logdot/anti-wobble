//! Simple patching library for windows processes.

use core::slice;

use mmap_rs::{MemoryAreas, Mmap, MmapOptions};

const CALL_BYTES: [u8; 8] = [0xff, 0x15, 0x02, 0x00, 0x00, 0x00, 0xeb, 0x08];
const NEAR_JUMP: [u8; 1] = [0xe9];

/// A struct representing a single patch done to the game's code.
/// A patch can be undone by calling `unpatch`.
pub struct Patch {
    size: usize,
    overwritten: Vec<u8>,
    mmap: Mmap,
}

impl Patch {
    /// Creates a patch at `address` so `function` can be run.
    /// `size` determines how many bytes are overwritten for call, must be at least 4.
    ///
    /// # Safety
    /// It is the responsibility of the caller to ensure that the inserted function is compatible with the original code.
    pub unsafe fn patch_call(address: usize, function: *const (), size: usize) -> Self {
        // Save the overwritten bytes
        let process_bytes = slice::from_raw_parts(address as *const u8, size);
        let overwritten = process_bytes.to_vec();

        let memory_cave = search_memory_cave(address).expect("No memory cave found");

        let mut mmap = MmapOptions::new(MmapOptions::page_size()).unwrap()
            .with_address(memory_cave)
            .map_mut().unwrap();

        let address = address as *mut u8;

        let mem = mmap.as_mut_ptr();

        // Write relative jump
        std::ptr::copy_nonoverlapping(NEAR_JUMP.as_ptr(), address, NEAR_JUMP.len());
        let jump_offset = mem as i32 - address as i32 - 5;
        std::ptr::copy_nonoverlapping(&jump_offset as *const _ as *const u8, address.add(1), 4);

        // Write nop slide
        let nops = vec![0x90; size - 5];
        std::ptr::copy_nonoverlapping(nops.as_ptr(), address.add(5), size - 5);

        // Keeps track of offset in memory
        let mut offset = 0;

        // Write the overwritten bytes to the memory
        std::ptr::copy_nonoverlapping(overwritten.as_ptr(), mem.add(offset), overwritten.len());
        offset += overwritten.len();

        // Write the call to the memory
        let function_ptr = &function as *const _ as *const u8;

        std::ptr::copy_nonoverlapping(CALL_BYTES.as_ptr(), mem.add(offset), CALL_BYTES.len());
        offset += CALL_BYTES.len();
        std::ptr::copy_nonoverlapping(function_ptr, mem.add(offset), 8);
        offset += 8;

        // Jump back to the original code
        std::ptr::copy_nonoverlapping(NEAR_JUMP.as_ptr(), mem.add(offset), NEAR_JUMP.len());
        let jump_offset = address.add(size) as i32 - mem.add(offset) as i32 - 5;
        offset += NEAR_JUMP.len();
        std::ptr::copy_nonoverlapping(&jump_offset as *const _ as *const u8, mem.add(offset), 4);

        let mmap = mmap.make_exec().unwrap();

        Patch {
            size,
            overwritten,
            mmap,
        }
    }
}

/// Searches for a valid memory region that can be used for code within the 4GB address space for a jump.
fn search_memory_cave(address: usize) -> Option<usize> {
    (address-0x80000000..address+0x80000000).step_by(MmapOptions::allocation_granularity()).find(|address| {
        MemoryAreas::query(*address).unwrap().is_none()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEAD_BEEF: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];

    fn dummy() {
        println!("Dummy function");
    }

    #[test]
    fn test_patch_call() {
        let address_space = DEAD_BEEF.to_vec().repeat(10);

        let address = address_space.as_ptr() as usize;
        let size = 10;

        let patch = unsafe { Patch::patch_call(address, dummy as *const (), size) };

        // Check that bytes successfully written into mmap
        let mmap = patch.mmap.as_ptr();
        let overwritten = unsafe { slice::from_raw_parts(mmap, size) };
        assert_eq!(*overwritten, DEAD_BEEF.to_vec().repeat(10)[..size])
    }
}
