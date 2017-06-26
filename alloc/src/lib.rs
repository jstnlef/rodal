#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate rodal;
use std::mem;
use std::ptr;
use std::ffi::{CString};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering, AtomicPtr};
use rodal::*;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
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





















/*struct UnsafeMut<T> {
    cell: UnsafeCell<T>
}
impl<T> UnsafeMut<T> {
    fn new(val: T) -> Self {
        UnsafeMut::<T> {
            cell: UnsafeCell::new(val),
        }
    }
}
impl<T> Deref for UnsafeMut<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe{mem::transmute(self.cell.get())}
    }
}
impl<T> DerefMut for UnsafeMut<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe{mem::transmute(self.cell.get())}
    }
}
unsafe impl<T> Sync for UnsafeMut<T> { }
unsafe impl<T> Send for UnsafeMut<T> { }

const FREE_NAME: &'static [u8] = b"free\0";
const REALLOC_NAME: &'static [u8] = b"realloc\0";
lazy_static!{
    static ref REAL_FREE: UnsafeMut<AtomicPtr<()>> = UnsafeMut::new(AtomicPtr::<()>::new(ptr::null_mut()));
    static ref REAL_REALLOC: UnsafeMut<AtomicPtr<()>>  = UnsafeMut::new(AtomicPtr::<()>::new(ptr::null_mut()));
    static ref DEALLOCATE_INIT_START: UnsafeMut<AtomicBool> = UnsafeMut::new(AtomicBool::new(false));
    static ref DEALLOCATE_INIT_FINISH: UnsafeMut<AtomicBool> = UnsafeMut::new(AtomicBool::new(false));
}

type FreeType = extern fn(*mut libc::c_void); // The actual types of the atomicptr
type ReallocType = extern fn(*mut libc::c_void, libc::size_t)->(*mut libc::c_void);

// YES I KNOW THIS IS not efficient
#[inline]
fn init_deallocate() {
    unsafe {
        // We havent initilised yet
        if DEALLOCATE_INIT_START.compare_and_swap(false, true, Ordering::SeqCst) == false {
            let free: *mut() = mem::transmute(libc::dlsym(libc::RTLD_NEXT, FREE_NAME.as_ptr() as *const libc::c_char));
            assert!(!free.is_null());
            REAL_FREE.store(free, Ordering::SeqCst);

            let realloc: *mut() = mem::transmute(libc::dlsym(libc::RTLD_NEXT, REALLOC_NAME.as_ptr() as *const libc::c_char));
            assert!(!realloc.is_null());
            REAL_REALLOC.store(free, Ordering::SeqCst);

            DEALLOCATE_INIT_FINISH.store(true, Ordering::SeqCst);
        }

        // Wait for deallocate init to finish
        while !DEALLOCATE_INIT_FINISH.load(Ordering::SeqCst) {}
    }
}

#[no_mangle]
pub unsafe extern fn free(ptr: *mut libc::c_void) {
    init_deallocate();

    if !is_rodal_dump(ptr) {
        (mem::transmute::<*mut(), FreeType>(REAL_FREE.load(Ordering::SeqCst)))(ptr);
    }
}

#[no_mangle]
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
        (mem::transmute::<*mut (), ReallocType>(REAL_REALLOC.load(Ordering::SeqCst)))(ptr, new_size)
    }
}*/