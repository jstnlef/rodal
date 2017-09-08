// Copyright 2017 The Australian National University
// 
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate libc;
use std::mem;
use super::*;
use std::sync::atomic::{Ordering, fence};

pub fn is_rodal_dump<T>(ptr: *const T) -> bool {
    let ptr = Address::from_ptr(ptr);
    match unsafe{RODAL_BOUND} {
        Some((start, end)) => start <= ptr && ptr < end,
        _ => false
    }
}

extern crate alloc;
extern crate alloc_system;
use self::alloc::heap::{Alloc, AllocErr, Layout, Excess, CannotReallocInPlace};

#[global_allocator]
pub static RODAL_ALLOC: RodalAlloc = RodalAlloc::new();

pub struct RodalAlloc{sys: std::cell::UnsafeCell<alloc_system::System>}

impl RodalAlloc {
    const fn new() -> RodalAlloc {
        RodalAlloc{sys: std::cell::UnsafeCell::new(alloc_system::System{})}
    }
    fn get_sys(&self) -> &mut alloc_system::System {
        unsafe { std::mem::transmute(self.sys.get())}
    }
}
unsafe impl Sync for RodalAlloc {}
unsafe impl Send for RodalAlloc {}

unsafe impl Alloc for RodalAlloc {
    #[inline] unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> { (&*self).alloc(layout) }
    #[inline] unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> { (&*self).alloc_zeroed(layout) }
    #[inline] unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) { (&*self).dealloc(ptr, layout) }
    #[inline] unsafe fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> Result<*mut u8, AllocErr> { (&*self).realloc(ptr, old_layout, new_layout) }
    #[inline] fn oom(&mut self, err: AllocErr) -> ! { (&*self).oom(err) }
}

unsafe impl<'a> Alloc for &'a RodalAlloc {
    #[inline] unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> { self.get_sys().alloc(layout) }

    #[inline] unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> { self.get_sys().alloc_zeroed(layout) }

    #[inline] unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if !is_rodal_dump(ptr) {
            self.get_sys().dealloc(ptr, layout);
        }
    }

    #[inline] unsafe fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> Result<*mut u8, AllocErr> {
        if is_rodal_dump(ptr) {
            if old_layout.size() >= new_layout.size() {
                Ok(ptr) // Allocated area is large enough
            } else {
                // Have to copy to a new (really allocated) area
                let res = self.alloc(new_layout.clone());
                if let Ok(new_ptr) = res {
                    let size = std::cmp::min(old_layout.size(), new_layout.size());
                    std::ptr::copy_nonoverlapping(ptr, new_ptr, size);
                }
                res
            }
        } else {
            self.get_sys().realloc(ptr, old_layout, new_layout)
        }
    }
    #[inline] fn oom(&mut self, err: AllocErr) -> ! {
        self.get_sys().oom(err)
    }
}