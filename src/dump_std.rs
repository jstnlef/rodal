/// Implements Dump for various standard library types
use super::*;
use std;

// are referencing the types declared here and not the real std types
rodal_pointer!(['a, T] &'a T = *T);
rodal_pointer!(['a, T] &'a mut T = *T);
rodal_pointer!([T] *const T = *T);
rodal_pointer!([T] * mut T = *T);
rodal_pointer!([T] std::sync::atomic::AtomicPtr<T> = *T);

rodal_object_reference!([T: Dump] std::boxed::Box<T> = &T);
rodal_object!([T: Dump] std::boxed::Box<[T]> = Repr<T>);

rodal_value!(std::sync::atomic::AtomicBool);
rodal_value!(std::sync::atomic::AtomicIsize);
rodal_value!(std::sync::atomic::AtomicUsize);

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
unsafe impl<T: Dump> Dump for std::option::Option<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::option::Option<T>", "dump");
        match self {
            &Some(ref val) => dumper.dump_object_here(val),
            &None => dumper.dump_value(self)
        }
    }
}
// These definitions are copied from the standard library
// This is neccesary so we can use private fields, and types
// that are unstable, by making copies whith identical layouts
// But these prefixes are prefixed with Rodal

// Note: the types declared here but without 'pub' are either private to the real standard libarary
// or are unstable
/// core::nonzero (libcore/nonzero.rs)
struct NonZero<T>(pub T); // T: core::nonzero::Zeroable

/// core::ptr (libcore/ptr.rs)
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

rodal_object_reference!([T: Dump] (Unique<T>) = &T);
rodal_object!([T: Dump] Unique<[T]> = Repr<T>);

/// core::ptr (libcore/ptr.rs)
struct Shared<T: ?Sized> { pub pointer: NonZero<*const T>, _marker: std::marker::PhantomData<T> }
rodal_object_reference!([T: Dump] Shared<T> = &T);
rodal_object!([T: Dump] Shared<[T]> = Repr<T>);

/// collections::vec (libcollections/vec.rs)
pub struct Vec<T> { buf: RawVec<T>, pub len: usize }
/// alloc::raw_vec (liballoc/rawvec.rs)
struct RawVec<T> { pub ptr: Unique<T>, pub cap: usize }
unsafe impl<T: Dump> Dump for std::vec::Vec<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::vec::Vec<T>", "dump");

        // Transmute to the fake_std version so we can access private fields
        let fake_self: &Vec<T> = unsafe{std::mem::transmute(self)};

        if std::mem::size_of::<T>()*fake_self.buf.cap == 0 {
            // Dosn't point to any real memory, so just dump a raw value
            dumper.dump_value(&fake_self.buf.ptr);
        } else {
            dumper.reference_object_function_sized_position(
                self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe { std::mem::transmute::<fn(&Vec<T>, &mut D), DumpFunction<D>>(Vec::<T>::dump_contents) },
                *fake_self.buf.ptr.as_ref(), // Where to actually dump the data
                std::mem::size_of::<T>() * fake_self.buf.cap, std::mem::align_of::<T>());
            dumper.dump_reference(fake_self.buf.ptr.as_ref());
        }

        // Dump the fields of the vector
        dumper.dump_object(&fake_self.buf.cap);
        dumper.dump_object(&fake_self.len);
    }
}
impl <T: Dump> Vec<T> {
    fn dump_contents<D: ? Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::vec::Vec<T>", "dump_contents");
        let real_self: &std::vec::Vec<T> = unsafe{mem::transmute(self)};
        dumper.set_position(Address::from_ptr(self.buf.ptr.as_ptr()));
        for val in real_self {
            dumper.dump_object(val);
        }
    }
}
//collections::string (src/libcollections/string.rs)
pub struct String { pub vec: std::vec::Vec<u8> }
rodal_struct!(std::string::String{vec} = String);

// alloc::arc (liballoc/arc.rs)
pub struct Arc<T: ?Sized> { ptr: Shared<ArcInner<T>>, }
rodal_struct!([T: Dump] std::sync::Arc<T>{ptr} = Arc<T>);

// alloc::arc (liballoc/arc.rs)
struct ArcInner<T: ?Sized> { strong: std::sync::atomic::AtomicUsize, weak: std::sync::atomic::AtomicUsize, data: T, }
rodal_struct!([T: Dump] ArcInner<T>{strong, weak, data});

// std::sys::poision (libstd/syscommon/poison.rs)
struct Flag { pub failed: std::sync::atomic::AtomicBool }
rodal_value!(Flag);

// core::cell (libcore/cell.rs)
pub struct UnsafeCell<T: ?Sized> { pub value: T }
rodal_struct!([T: ?Sized + Dump] UnsafeCell<T>{value});

// std::sync (libstd/sync/rwlock.rs)
pub struct RwLock<T: ?Sized> {
    pub inner: Box<self::sys::RWLock>, // sys::RWLock (system dependent struct, the exact value can't be dumped be we can create a new one and dump that instead)
    poison: Flag,
    pub data: UnsafeCell<T>,
}
// Acquires a read lock on it's contents before it dumps
unsafe impl<T: ?Sized + Dump> Dump for std::sync::RwLock<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::sync::RwLock<T>", "dump");
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

// System specific stuff
#[cfg(windows)]
mod sys {
    use libc;
    // std::sys::rwlock (libstd/sys/windows/rwlock.rs)
    pub struct RWLock { pub inner: super::UnsafeCell<SRWLOCK> }

    // std::sys::c (libstd/sys/windows/c.rs)
    #[repr(C)]
    pub struct SRWLOCK { pub ptr: LPVOID }
    pub type LPVOID = *mut libc::c_void;
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
}
//pub struct sys::RWLock { pub inner: super::UnsafeCell<SRWLOCK> }
unsafe impl Dump for sys::RWLock {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("sys::RWLock", "dump");

        // Create a new std::sync::RwLock, and dump its value of inner
        // (so that when we load the dump the RwLock will have it's initial state)
        let lock: RwLock<()> = unsafe{ std::mem::transmute(std::sync::RwLock::<()>::new(())) };
        dumper.dump_value_here(lock.inner.as_ref());
    }
}

// std::collections::hash_map (src/libstd/collections/hash/map.rs)
pub struct RandomState { pub k0: u64, pub k1: u64, }
rodal_struct!(std::collections::hash_map::RandomState{k0, k1} = RandomState);
struct DefaultResizePolicy;
rodal_struct!(DefaultResizePolicy{});
pub struct HashMap<K, V, S = RandomState> {
    hash_builder: S,
    table: RawTable<K, V>,
    resize_policy: DefaultResizePolicy,
}
// The Eq + Hash and BuildHasher contstratins are needed
// as almost all of the hashmap's code requires this
// (without them, we won't even be able to iterate over it's elements)
unsafe impl<K: Eq + std::hash::Hash + Dump, V: Dump, S: std::hash::BuildHasher + Dump> Dump
for std::collections::HashMap<K, V, S> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::collections::HashMap<K, V, S>", "dump");
        // Transmute to the fake_std version so we can access private fields
        let fake_self: &HashMap<K, V, S> = unsafe{std::mem::transmute(self)};

        dumper.dump_object_here(&fake_self.hash_builder);

        // Dump table
        dumper.dump_object_here(&fake_self.table.capacity_mask);
        dumper.dump_object_here(&fake_self.table.size);
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
            dumper.reference_object_function_sized_position(
                fake_self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe{std::mem::transmute::<fn(&HashMap<K, V, S>, &mut D), DumpFunction<D>>(
                    HashMap::<K, V, S>::dump_contents)},
                pos.to_ref::<HashUint>(), // Where to actually dump the data
                size, align);
            dumper.dump_object_here(&fake_self.table.hashes);
        }
        dumper.dump_object_here(&fake_self.resize_policy);
    }
}
impl<K: Eq + std::hash::Hash + Dump, V: Dump, S: std::hash::BuildHasher + Dump>
HashMap<K, V, S> {
    fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("std::collections::HashMap<K, V, S>", "dump_contents");
        let real_pos = unsafe{&*self.table.hashes.ptr()};
        dumper.set_position(Address::new(real_pos));
        // Dump the stored hashes
        dumper.dump_value_sized(real_pos, self.table.capacity()*std::mem::size_of::<HashUint>());

        // Create a list to hold the positions and dump functions of the tables contents
        // (in case iteration dosn't occur in memory order)
        let mut list = DumpList::<D>::new();

        let real_self: &std::collections::HashMap<K, V, S> = unsafe{std::mem::transmute(self)};
        // Record each element of the table in the list
        for (key, value) in real_self {
            list.add(key);
            list.add(value);
        }
        // Dump the tables contents
        list.dump(dumper);
    }
}

// std::collections::hash_map (src/libstd/collections/hash/table.rs)
struct TaggedHashUintPtr(Unique<HashUint>);

// Note: this is a tagged pointer, but it will either have the value
// of the underlying pointer or the pointer + 1
// Either way the dumper will preserve the tag,
// and it will point within the tables memory, so the Dumper will store it properly
rodal_pointer!(TaggedHashUintPtr = *HashUint);
impl TaggedHashUintPtr {
    fn ptr(&self) -> *mut HashUint { (self.0.as_ptr() as usize & !1) as *mut HashUint }
}
type HashUint = usize;
struct RawTable<K, V> {
    pub capacity_mask: usize,
    pub size: usize,
    pub hashes: TaggedHashUintPtr,
    pub marker: std::marker::PhantomData<(K, V)>,
}
impl<K, V> RawTable<K, V> {
    fn capacity(&self) -> usize { self.capacity_mask.wrapping_add(1) }
}

// Slices...

//core::slice (src/libcore/slice/mod.rs)
#[repr(C)] // Repr<T> has the same layout as &[T]
struct Repr<T> {
    pub data: *const T,
    pub len: usize,
}
unsafe impl<T: Dump> Dump for Repr<T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("Repr<T>", "dump");
        if std::mem::size_of::<T>()*self.len == 0 {
            // Dosn't point to any real memory, so just dump a raw value
            dumper.dump_value(&self.data);
        } else {
            dumper.reference_object_function_sized_position(
                self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe { std::mem::transmute::<fn(&Repr<T>, &mut D), DumpFunction<D>>(Repr::<T>::dump_contents) },
                unsafe{self.data.as_ref().unwrap()}, // Where to actually dump the data
                std::mem::size_of::<T>() *self.len, std::mem::align_of::<T>());

            dumper.dump_reference(unsafe{mem::transmute::<&*const T, &&T>(&self.data)});
        }

        dumper.dump_object(&self.len);
    }
}
impl <T: Dump> Repr<T> {
    fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("Repr<T>", "dump_contents");
        let real_self: &&[T] = unsafe{mem::transmute(self)};

        dumper.set_position(Address::from_ptr(self.data));
        // Dump the contents of the slice
        for val in *real_self {
            dumper.dump_object(val)
        }
    }
}


rodal_struct!(['a, T] &'a [T]{data, len} = Repr<T>);
rodal_struct!(['a, T] &'a mut[T]{data, len} = Repr<T>);
rodal_struct!([T] *const[T]{data, len} = Repr<T>);
rodal_struct!([T] *mut[T]{data, len} = Repr<T>);

// Use new() to make a new fake arc and you can dump that
// You can then reload a pointer to it as a real Arc
pub struct FakeArc<'a, T: 'a> {
    inner: &'a T,
}
//struct ArcInner<T: ?Sized> { strong: std::sync::atomic::AtomicUsize, weak: std::sync::atomic::AtomicUsize, data: T, }
impl<'a, T: Dump> FakeArc<'a, T> {
    pub fn new(val: &'a T) -> FakeArc<'a, T> {
        FakeArc::<'a, T> {
            inner: val
        }
    }

    // Dump an ArcInner
    fn dump_inner<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("FakeArc<'a, T>", "dump_inner");

        let start = dumper.current_position();
        // Dump the default strong value of 1
        dumper.dump_padding((start + offset_of!(ArcInner<T> => strong).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(&std::sync::atomic::AtomicUsize::new(1));

        // Dump the default weak value of 1
        dumper.dump_padding((start + offset_of!(ArcInner<T> => weak).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(&std::sync::atomic::AtomicUsize::new(1));

        // Dump the containing data
        dumper.dump_padding((start + offset_of!(ArcInner<T> => data).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(self.inner);
    }
}
unsafe impl<'a, T: Dump> Dump for FakeArc<'a, T> {
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record("FakeArc<'a, T>", "dump");

        // Where to dump our fake ArcInner
        // (make it include the real position of inner so that references to inside of inner will be correctly preserved)
        let fake_inner = (Address::new(self.inner) - offset_of!(ArcInner<T> => data).get_byte_offset()).to_ref::<ArcInner<T>>();

        dumper.reference_object_function_sized_position(
            self, // the argument to pass to the dump function
            // The function to use to dump the contents
            unsafe { std::mem::transmute::<fn(&FakeArc<'a, T>, &mut D), DumpFunction<D>>(FakeArc::<'a, T>::dump_inner) },
            fake_inner, std::mem::size_of::<ArcInner<T>>(), std::mem::align_of::<ArcInner<T>>());

        dumper.dump_padding(&self.inner);
        dumper.dump_reference_here(&fake_inner);
    }
}