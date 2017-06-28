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
use std::ptr;
use std::ffi::{CString};
use std::collections::HashMap;
use super::*;

pub unsafe fn load_asm_bounds(start: Address, end: Address) {
    RODAL_BOUND = Some((start, end));
}
pub unsafe fn load_asm_pointer_move<'a, T>(ptr: *mut T) -> T {
    ptr::read(ptr)
}

pub fn try_load_asm_name_move<T>(name: &str) -> Option<T> {
    let rtld_default = unsafe {libc::dlopen(ptr::null(), 0)};
    let cstring = CString::new(name.to_string());
    let ret = unsafe {libc::dlsym(rtld_default, cstring.unwrap().as_ptr())};
    if unsafe{libc::dlerror().is_null()} || ret.is_null() {
        None
    } else {
        Some(unsafe{ptr::read(mem::transmute::<*const libc::c_void, *mut T>(ret))})
    }
}
pub fn load_asm_name_move<T>(name: &str) -> T {
    let rtld_default = unsafe {libc::dlopen(ptr::null(), 0)};
    let cstring = CString::new(name.to_string());
    let ret = unsafe {libc::dlsym(rtld_default, cstring.unwrap().as_ptr())};
    unsafe{ptr::read(mem::transmute::<*const libc::c_void, *mut T>(ret))}
}
pub fn load_asm_tags<'a>()->HashMap<usize, Vec<*const ()>> { load_asm_name_move("RODAL_TAGS") }

pub static mut RODAL_BOUND: Option<(Address, Address)> = None;
