use libc;
use std::mem;
use std::ptr;
use std::ffi::{CString};
use super::*;


pub unsafe fn load_asm_pointer<'a, T: ?Sized>(ptr: *mut T) -> &'a T {
    RODAL_END = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, RODAL_END_NAME.as_ptr())));
    RODAL_START = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, RODAL_START_NAME.as_ptr())));

    mem::transmute(ptr)
}
pub fn load_asm_name<'a, T>(name: &str) -> &'a T {
    unsafe {RODAL_END = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, RODAL_END_NAME.as_ptr())))};
    unsafe {RODAL_START = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, RODAL_START_NAME.as_ptr())))};

    let rtld_default = unsafe {libc::dlopen(ptr::null(), 0)};
    let cstring = CString::new(name.to_string());
    let ret = unsafe {libc::dlsym(rtld_default, cstring.unwrap().as_ptr())};
    assert!(unsafe {libc::dlerror().is_null()});

    unsafe { mem::transmute::<*const libc::c_void, &T>(ret) }
}
pub fn load_asm_tags<'a>() -> &'a Vec<(usize, Vec<&'a ()>)> { load_asm_name("RODAL_TAGS") }

static mut RODAL_END: Option<Address> = None; // End of the rodal data area
static mut RODAL_START: Option<Address> = None; // Start of the rodal data area
static mut REAL_FREE: Option<extern fn(*mut libc::c_void)> = None;
static mut REAL_REALLOC: Option<extern fn(*mut libc::c_void, libc::size_t)->(*mut libc::c_void)> = None;

fn is_rodal_dump(ptr: *const libc::c_void) -> bool {
    let ptr = Address::from_ptr(ptr);
    let end = unsafe{RODAL_END.unwrap_or(Address::null())};
    let start = unsafe{RODAL_START.unwrap_or(Address::max())};
    start <= ptr && ptr < end
}

const RODAL_END_NAME: &'static [u8] = b"RODAL_END\0";
const RODAL_START_NAME: &'static [u8] = b"RODAL_START\0";
const FREE_NAME: &'static [u8] = b"free\0";
const REALLOC_NAME: &'static [u8] = b"realloc\0";

#[inline]
unsafe fn init_deallocate() {
    if REAL_FREE.is_none() {
        REAL_FREE = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, FREE_NAME.as_ptr())));
        assert!(REAL_FREE.is_some());

        REAL_REALLOC = Some(mem::transmute(libc::dlsym(libc::RTLD_NEXT, REALLOC_NAME.as_ptr())));
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