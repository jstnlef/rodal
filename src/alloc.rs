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

fn is_rodal_dump(ptr: *const libc::c_void) -> bool {
    let ptr = Address::from_ptr(ptr);
    match unsafe{RODAL_BOUND} {
        Some((start, end)) => start <= ptr && ptr < end,
        _ => false
    }
}

const FREE_NAME: &'static [u8] = b"free\0";
const REALLOC_NAME: &'static [u8] = b"realloc\0";
static mut REAL_FREE: Option<extern fn(*mut libc::c_void)> = None;
static mut REAL_REALLOC: Option<extern fn(*mut libc::c_void, libc::size_t)->(*mut libc::c_void)> = None;

// This should be the first hing called in main
#[no_mangle]
pub unsafe extern fn rodal_init_deallocate() {
    REAL_FREE = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, FREE_NAME.as_ptr() as *const libc::c_char)));
    assert!(REAL_FREE.is_some());
    REAL_REALLOC = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, REALLOC_NAME.as_ptr() as *const libc::c_char)));
    assert!(REAL_REALLOC.is_some());

    // Make sure other threads (when they start) see the writes to the global variables
    fence(Ordering::SeqCst);
}
#[no_mangle]
pub unsafe extern fn rodal_free(ptr: *mut libc::c_void) {
    if !is_rodal_dump(ptr) {
        (REAL_FREE.unwrap())(ptr);
    }
}
#[no_mangle]
pub unsafe extern fn rodal_realloc(ptr: *mut libc::c_void, new_size: libc::size_t)->*mut libc::c_void {
    if is_rodal_dump(ptr) {
        let old_size = *(Address::from_ptr(ptr) - mem::size_of::<libc::size_t>()).to_ref::<usize>();
        if old_size >= new_size {
            ptr // Allocated area is large enough
        } else {
            // Have to copy to a new (really malloced) area
            libc::memcpy(libc::malloc(new_size), ptr, old_size)
        }
    } else {
        (REAL_REALLOC.unwrap())(ptr, new_size)
    }
}