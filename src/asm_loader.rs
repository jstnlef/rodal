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
pub fn load_asm_name_move<T>(name: &str) -> Option<T> {
    let rtld_default = unsafe {libc::dlopen(ptr::null(), 0)};
    let cstring = CString::new(name.to_string());
    let ret = unsafe {libc::dlsym(rtld_default, cstring.unwrap().as_ptr())};
    Some(unsafe{ptr::read(mem::transmute::<*const libc::c_void, *mut T>(ret))})
}
pub fn load_asm_tags<'a>()->HashMap<usize, Vec<*const ()>> { load_asm_name_move("RODAL_TAGS").unwrap() }

pub static mut RODAL_BOUND: Option<(Address, Address)> = None;