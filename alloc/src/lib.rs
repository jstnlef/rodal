extern crate libc;
extern crate rodal;
use std::mem;
use rodal::*;
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
#[inline]
unsafe fn init_deallocate() {
    if REAL_FREE.is_none() {
        REAL_FREE = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, FREE_NAME.as_ptr() as *const libc::c_char)));
        assert!(REAL_FREE.is_some());
        REAL_REALLOC = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, REALLOC_NAME.as_ptr() as *const libc::c_char)));
        assert!(REAL_REALLOC.is_some());
    }
}
#[no_mangle]
#[allow(dead_code)]
pub unsafe extern fn free(ptr: *mut libc::c_void) {
    init_deallocate();
    if !is_rodal_dump(ptr) {
        (REAL_FREE.unwrap())(ptr);
    }
}
#[no_mangle]
#[allow(dead_code)]
pub unsafe extern fn realloc(ptr: *mut libc::c_void, new_size: libc::size_t)->*mut libc::c_void {
    init_deallocate();
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