// Copyright 2014 The Rust Project Developers.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
// Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
// WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

/// Implements Dump for various standard library types
use super::*;
use std;

rodal_pointer!(['a, T: Named] &'a T = *T [type_name!("&{}", T)]);
rodal_pointer!(['a, T: Named] &'a mut T = *T [type_name!("&mut {}", T)]);
rodal_pointer!([T: Named] *const T = *T [type_name!("*const {}", T)]);
rodal_pointer!([T: Named] * mut T = *T [type_name!("*mut {}", T)]);
rodal_pointer!([T: Named] std::sync::atomic::AtomicPtr<T> = *T [type_name!("std::sync::atomic::AtomicPtr<{}>", T)]);

rodal_object_reference!([T: Dump] std::boxed::Box<T> = &T [type_name!("std::boxed::Box<{}>", T)]);
rodal_object!([T: Dump] std::boxed::Box<[T]> = Repr<T> [type_name!("std::boxed::Box<[{}]>", T)]);
rodal_object!(std::boxed::Box<str> = Repr<u8>);

rodal_value!(std::sync::atomic::AtomicBool);
rodal_value!(std::sync::atomic::AtomicIsize);
rodal_value!(std::sync::atomic::AtomicUsize);
rodal_value!([T: ?Sized + Named] std::marker::PhantomData<T> [type_name!("std::marker::PhantomData<{}>", T)]); // Should be empty

// Primitives, not declared here
rodal_value!(bool);
rodal_value!(i16);
rodal_value!(i32);
rodal_value!(i64);
rodal_value!(i8);
rodal_value!(isize);
rodal_value!(u16);
rodal_value!(u32);
rodal_value!(u64);
rodal_value!(u8);
rodal_value!(usize);
rodal_value!(f32);
rodal_value!(f64);
rodal_value!(char);

//rodal_enum!([T: Dump] std::option::Option<T>{None, (Some: val)});
// This is implemented manually as the rodal_enum! macro dosn't work with generics...
rodal_named!([T: Named] std::option::Option<T> [type_name!("std::option::Option<{}>", T)]);
unsafe impl<T: Dump> Dump for std::option::Option<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        match self {
            &Some(ref val) => {
                dumper.dump_prefix_value(val);
                dumper.dump_object(val);
                dumper.dump_suffix_value(self);
            },
            &None => dumper.dump_value(self)
        }
    }
}

// The types declared here have been copied (and slightly modified) from the rust source code
// This is neccesary so we can use private fields, and types, that are unstable, by making copies whith identical layouts.
// Types referenced without an 'std::' prefix, are the copies defined in this file, and not the real ones.

/// unstable core::nonzero (libcore/nonzero.rs)
struct NonZero<T>(pub T); // T: core::nonzero::Zeroable

/// unstable core::ptr (libcore/ptr.rs)
struct Unique<T: ?Sized> {pub pointer: NonZero<*const T>, pub _marker: std::marker::PhantomData<T>}
// Utility impls to make unique more usable
#[allow(dead_code)]
impl<T> Unique<T> {
    pub fn clone(&self) -> Unique<T> { unsafe{std::mem::transmute_copy(self)} }
    pub fn as_ref_mut(&mut self) ->&mut &T { unsafe{std::mem::transmute(self)} }
    pub fn as_ref(&self) ->&&T { unsafe{std::mem::transmute(self)} }
    pub fn as_ptr_mut(&mut self) ->*mut T { unsafe{std::mem::transmute_copy(self)} }
    pub fn as_ptr(&self) -> *const T { unsafe{std::mem::transmute_copy(self)} }
}

rodal_object_reference!([T: ?Sized + Dump] (Unique<T>) = &T [type_name!("core::ptr::Unique<{}>", T)]);
rodal_object!([T: Dump] Unique<[T]> = Repr<T> [type_name!("coreptr::Unique<[{}]>", T)]);
rodal_object!(Unique<str> = Repr<u8>);

/// unstable core::ptr (libcore/ptr.rs)
struct Shared<T: ?Sized> { pub pointer: NonZero<*const T>, _marker: std::marker::PhantomData<T> }
rodal_object_reference!([T: ?Sized + Dump] Shared<T> = &T [type_name!("core::ptr::Shared<{}>", T)]);
rodal_object!([T: Dump] Shared<[T]> = Repr<T> [type_name!("core::ptr::Shared<[{}]>", T)]);
rodal_object!(Shared<str> = Repr<u8>);

// public collections::vec (libcollections/vec.rs)
pub struct Vec<T> { buf: RawVec<T>, pub len: usize }
// unstable alloc::raw_vec (liballoc/rawvec.rs)
/* NOTE: in rust v1.21 the definition was change to:
pub struct RawVec<T, A: Alloc = Heap> {
    ptr: Unique<T>,
    cap: usize,
    a: A,
}
However, it is currently only used be Vec (which dosn't provide an ovveride for 'A'), and 'Heap' is an empty struct
*/
struct RawVec<T> { pub ptr: Unique<T>, pub cap: usize }
rodal_named!([T: Named] std::vec::Vec<T> [type_name!("std::vec::Vec<{}>", T)]);
unsafe impl<T: Dump> Dump for std::vec::Vec<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");

        // Transmute to the fake_std version so we can access private fields
        let fake_self: &Vec<T> = unsafe{std::mem::transmute(self)};

        if std::mem::size_of::<T>()*fake_self.buf.cap == 0 {
            // Dosn't point to any real memory, so just dump a raw value
            dumper.dump_value(&fake_self.buf.ptr);
        } else {
            dumper.dump_reference_object_function_sized_position(
                self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe { std::mem::transmute::<fn(&Vec<T>, &mut D), DumpFunction<D>>(Vec::<T>::dump_contents) },
                fake_self.buf.ptr.as_ref(), // Where to actually dump the data
                std::mem::size_of::<T>() * fake_self.buf.cap, std::mem::align_of::<T>());
        }

        // Dump the fields of the vector
        dumper.dump_object(&fake_self.buf.cap);
        dumper.dump_object(&fake_self.len);
    }
}

rodal_named!([T: Named] Vec<T> [type_name!("std::vec::Vec<{}>", T)]);
impl <T: Dump> Vec<T> {
    fn dump_contents<D: ? Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump_contents");
        let real_self: &std::vec::Vec<T> = unsafe{mem::transmute(self)};
        dumper.set_position(Address::from_ptr(self.buf.ptr.as_ptr()));
        for val in real_self {
            dumper.dump_object(val);
        }
    }
}
// public collections::string (src/libcollections/string.rs)
pub struct String { pub vec: std::vec::Vec<u8> }
rodal_struct!(std::string::String{vec} = String);

// public alloc::arc (liballoc/arc.rs)
pub struct Arc<T: ?Sized> { ptr: Shared<ArcInner<T>>, }
rodal_struct!([T: ?Sized + Dump] std::sync::Arc<T>{ptr} = Arc<T> [type_name!("std::sync::Arc<{}>", T)]);

// private alloc::arc (liballoc/arc.rs)
pub struct ArcInner<T: ?Sized> { pub strong: std::sync::atomic::AtomicUsize, pub weak: std::sync::atomic::AtomicUsize, pub data: T, }
rodal_struct!([T: ?Sized + Dump] ArcInner<T>{strong, weak, data} [type_name!("alloc::arc<{}>", T)]);

// private std::sys::poision (libstd/syscommon/poison.rs)
struct Flag { pub failed: std::sync::atomic::AtomicBool }
rodal_value!(Flag);

// public core::cell (libcore/cell.rs)
pub struct UnsafeCell<T: ?Sized> { pub value: T }
rodal_struct!([T: ?Sized + Dump] UnsafeCell<T>{value} [type_name!("std::cell::UnsafeCell<{}>", T)]);

// public std::sync (libstd/sync/rwlock.rs)
pub struct RwLock<T: ?Sized> {
    pub inner: Box<self::sys::RWLock>, // sys::RWLock (system dependent struct, the exact value can't be dumped be we can create a new one and dump that instead)
    poison: Flag,
    pub data: UnsafeCell<T>,
}

// Acquires a read lock on it's contents before it dumps
rodal_named!([T: ?Sized + Named] std::sync::RwLock<T> [type_name!("std::sync::RwLock<{}>", T)]);
unsafe impl<T: ?Sized + Dump> Dump for std::sync::RwLock<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        use std::ops::Deref;
        // Acquire a read lock to self (just so no one tries to modify the contents whilst we try and dump it)
        let lock = self.read().unwrap();
        let data: &T = lock.deref();
        let fake_self: &RwLock<T> = unsafe{std::mem::transmute(self)};

        dumper.dump_object(&fake_self.inner);
        dumper.dump_object(&fake_self.poison);
        dumper.dump_object(data);
    }
}


// std::sync (src/libstd/sync/mutex.rs)
pub struct Mutex<T: ?Sized> {
    inner: Box<sys::Mutex>,
    poison: Flag,
    pub data: UnsafeCell<T>,
}

rodal_named!([T: ?Sized + Named] std::sync::Mutex<T> [type_name!("std::sync::Mutex<{}>", T)]);
unsafe impl<T: ?Sized + Dump> Dump for std::sync::Mutex<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        use std::ops::Deref;
        // Acquire a lock to self (just so no one tries to modify the contents whilst we try and dump it)

        let lock = self.lock().unwrap();
        let data: &T = lock.deref();
        let fake_self: &Mutex<T> = unsafe{std::mem::transmute(self)};

        dumper.dump_object(&fake_self.inner);
        dumper.dump_object(&fake_self.poison);
        dumper.dump_object(data);
    }
}

// private std (libstd/sys/)
#[cfg(windows)]
mod sys {
    use libc;
    // std::sys::rwlock (libstd/sys/windows/rwlock.rs)
    pub struct RWLock { pub inner: super::UnsafeCell<SRWLOCK> }

    // std::sys::c (libstd/sys/windows/c.rs)
    #[repr(C)]
    pub struct SRWLOCK { pub ptr: LPVOID }
    pub type LPVOID = *mut libc::c_void;

    pub struct Mutex {
        lock: AtomicUsize,
        held: UnsafeCell<bool>,
    }
}
#[cfg(target_os = "redox")]
mod sys {
    use libc;
    use std;
    // std::sys::rwlock (libstd/sys/redox/rwlock.rs)
    pub struct RWLock { pub mutex: Mutex }
    // std::sys::mutex (libstd/sys/redox/mutex.rs)
    pub struct Mutex { pub lock: super::UnsafeCell<i32> }
}
#[cfg(unix)]
mod sys {
    use libc;
    use std;
    // std::sys::rwlock (libstd/sys/unix/rwlock.rs)
    pub struct RWLock {
        pub inner: super::UnsafeCell<libc::pthread_rwlock_t>,
        pub write_locked: super::UnsafeCell<bool>,
        pub num_readers: std::sync::atomic::AtomicUsize,
    }
    pub struct Mutex { pub inner: super::UnsafeCell<libc::pthread_mutex_t> }
}

rodal_named!(sys::RWLock);
unsafe impl Dump for sys::RWLock {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");

        // Create a new std::sync::RwLock, and dump its value of inner
        // (so that when we load the dump the RwLock will have it's initial state)
        let lock: RwLock<()> = unsafe{ std::mem::transmute(std::sync::RwLock::<()>::new(())) };
        dumper.dump_value_here(lock.inner.as_ref());
    }
}

rodal_named!(sys::Mutex);
unsafe impl Dump for sys::Mutex {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");

        // Create a new std::sync::RwLock, and dump its value of inner
        // (so that when we load the dump the RwLock will have it's initial state)
        let mutex: Mutex<()> = unsafe{ std::mem::transmute(std::sync::Mutex::<()>::new(())) };
        dumper.dump_value_here(&*mutex.inner);
    }
}
// public std::collections::hash_map (src/libstd/collections/hash/map.rs)
pub struct RandomState { pub k0: u64, pub k1: u64, }
rodal_struct!(std::collections::hash_map::RandomState{k0, k1} = RandomState);
pub struct DefaultResizePolicy;
rodal_struct!(DefaultResizePolicy{});
pub struct HashMap<K, V, S = std::collections::hash_map::RandomState> {
    pub hash_builder: S,
    pub table: RawTable<K, V>,
    pub resize_policy: DefaultResizePolicy,
}

// The Eq + Hash and BuildHasher contstratins are needed
// as almost all of the hashmap's code requires this
// (without them, we won't even be able to iterate over it's elements)

rodal_named!([K: Eq + std::hash::Hash + Named, V: Named, S: std::hash::BuildHasher + Named] std::collections::HashMap<K, V, S> [type_name!("std::collections::HashMap<{}, {}, {}>", K, V, S)]);
unsafe impl<K: Eq + std::hash::Hash + Dump, V: Dump, S: std::hash::BuildHasher + Dump> Dump
for std::collections::HashMap<K, V, S> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        // Transmute to the fake_std version so we can access private fields
        let fake_self: &HashMap<K, V, S> = unsafe{std::mem::transmute(self)};

        dumper.dump_object(&fake_self.hash_builder);

        // Dump table
        dumper.dump_object(&fake_self.table.capacity_mask);
        dumper.dump_object(&fake_self.table.size);
        if fake_self.table.capacity() == 0 {
            // Not an actual pointer (there is no associated memory)
            dumper.dump_value(&fake_self.table.hashes);
        } else {
            // Compute the size and alignment of the associated memory area
            // (this was adapted from the real std's RawTable's Drop function)
            let hashes_size = fake_self.table.capacity()*std::mem::size_of::<HashUint>();
            let hash_align = std::mem::align_of::<HashUint>();
            let pairs_size = fake_self.table.capacity()*std::mem::size_of::<(K, V)>();
            let pair_align = std::mem::align_of::<(K, V)>();

            // Rounds up hash_size to be a multiple of pairs_align (this works as pairs_align is a power of 2)
            let pairs_offset = (hashes_size + pair_align - 1) & !(pair_align - 1);

            let size = pairs_offset + pairs_size;
            let align = std::cmp::max(hash_align, pair_align);

            let pos = super::Address::new(unsafe{&*fake_self.table.hashes.ptr()});
            dumper.dump_padding(&fake_self.table.hashes);
            dumper.dump_reference_object_function_sized_position_offset_here(
                fake_self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe{std::mem::transmute::<fn(&HashMap<K, V, S>, &mut D), DumpFunction<D>>(
                    HashMap::<K, V, S>::dump_contents)},
                &pos.to_ref::<HashUint>(), // Where to actually dump the data
                size, align, fake_self.table.hashes.tag() as isize);
        }
        dumper.dump_object(&fake_self.resize_policy);
    }
}
rodal_named!([K: Eq + std::hash::Hash + Named, V: Named, S: std::hash::BuildHasher + Named] HashMap<K, V, S> [type_name!("std::collections::HashMap<{}, {}, {}>", K, V, S)]);
impl<K: Eq + std::hash::Hash + Dump, V: Dump, S: std::hash::BuildHasher + Dump>
HashMap<K, V, S> {
    fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump_contents");
        let real_pos = unsafe{&*self.table.hashes.ptr()};
        dumper.set_position(Address::new(real_pos));
        // Dump the stored hashes
        dumper.dump_value_sized(real_pos, self.table.capacity()*std::mem::size_of::<HashUint>());

        let real_self: &std::collections::HashMap<K, V, S> = unsafe{std::mem::transmute(self)};


        // WARNING: We're assuming here that iteration happens in memory order
        // (so far it has worked, but if it dosn,t try using the comment out code bellow instead)
        for (key, value) in real_self {
            dumper.dump_object(key); // Assuming eveything is stored in this order
            dumper.dump_object(value);
        }
    }
}

// private std::collections::hash_map (src/libstd/collections/hash/table.rs)
pub struct TaggedHashUintPtr(Unique<HashUint>);

// Note: this is a tagged pointer, but it will either have the value
// of the underlying pointer or the pointer + 1
// Either way the dumper will preserve the tag,
// and it will point within the tables memory, so the Dumper will store it properly
rodal_pointer!(TaggedHashUintPtr = *HashUint);
impl TaggedHashUintPtr {
    #[inline] fn tag(&self) -> bool {
        (self.0.as_ptr() as usize) & 1 == 1
    }
    #[inline] fn ptr(&self) -> *mut HashUint { (self.0.as_ptr() as usize & !1) as *mut HashUint }
}
// private std::collections::hash_map (src/libstd/collections/hash/table.rs)
pub type HashUint = usize;

// private std::collections::hash_map (src/libstd/collections/hash/table.rs)
pub struct RawTable<K, V> {
    pub capacity_mask: usize,
    pub size: usize,
    pub hashes: TaggedHashUintPtr,
    pub marker: std::marker::PhantomData<(K, V)>,
}
impl<K, V> RawTable<K, V> {
    pub fn capacity(&self) -> usize { self.capacity_mask.wrapping_add(1) }
}


// Linked list
// std::collections::linked_list (src/libcollections/linked_list.rs)
pub struct LinkedList<T> {
    head: Shared<Node<T>>, //Option
    tail: Shared<Node<T>>, //Option
    len: usize,
    marker: std::marker::PhantomData<Box<Node<T>>>,
}
rodal_struct!([T: Dump] std::collections::linked_list::LinkedList<T>{head, tail, len, marker} = LinkedList<T> [type_name!("std::collections::linked_list::LinkedList<{}>", T)]);

// private std::collections::linked_list (src/libcollections/linked_list.rs)
struct Node<T> {
    next: Shared<Node<T>>, // Option
    prev: Shared<Node<T>>, // Option
    element: T,
}
rodal_struct!([T: Dump] Node<T>{next, prev, element} [type_name!("collections::linked_list::Node<{}>", T)]);

//private core::slice (src/libcore/slice/mod.rs)
#[repr(C)] // Repr<T> has the same layout as &[T]
struct Repr<T> {
    pub data: *const T,
    pub len: usize,
}
rodal_named!([T: Named] Repr<T> [type_name!("core::slice::Repr<{}>", T)]);
unsafe impl<T: Dump> Dump for Repr<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        if std::mem::size_of::<T>()*self.len == 0 {
            // Dosn't point to any real memory, so just dump a raw value
            dumper.dump_value(&self.data);
        } else {
            dumper.dump_reference_object_function_sized_position(
                self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe { std::mem::transmute::<fn(&Repr<T>, &mut D), DumpFunction<D>>(Repr::<T>::dump_contents) },
                unsafe{mem::transmute::<&*const T, &&T>(&self.data)}, // Where to actually dump the data
                std::mem::size_of::<T>() *self.len, std::mem::align_of::<T>());
        }

        dumper.dump_object(&self.len);
    }
}
impl <T: Dump> Repr<T> {
    fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump_contents");
        let real_self: &&[T] = unsafe{mem::transmute(self)};

        dumper.set_position(Address::from_ptr(self.data));
        // Dump the contents of the slice
        for val in *real_self {
            dumper.dump_object(val)
        }
    }
}

rodal_struct!(['a, T: Dump] &'a [T]{data, len} = Repr<T> [type_name!("&[{}]", T)]);
rodal_struct!(['a, T: Dump] &'a mut [T]{data, len} = Repr<T> [type_name!("&mut [{}]", T)]);
rodal_struct!([T: Dump] *const [T]{data, len} = Repr<T> [type_name!("*const [{}]", T)]);
rodal_struct!([T: Dump] *mut [T]{data, len} = Repr<T> [type_name!("*mut [{}]", T)]);

rodal_struct!(['a] &'a str{data, len} = Repr<u8> ["& str".to_string()]);
rodal_struct!(['a] &'a mut str{data, len} = Repr<u8> ["&mut str".to_string()]);
rodal_struct!(*const str{data, len} = Repr<u8>);
rodal_struct!(*mut str{data, len} = Repr<u8>);

// Just giving things names
rodal_named!([T: Named] std::thread::JoinHandle<T> [type_name!("std::thread::JoinHandle<{}>", T)]);