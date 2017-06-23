/// Implements Dump for various standard library types
use rodal;
use rodal::*;
use std;
use std::sync::atomic::*;

rodal_pointer!(['a, T: ?Sized] &'a T = *T);
rodal_pointer!(['a, T: ?Sized] &'a mut T = *T);
rodal_pointer!([T: ?Sized] *const T = *T);
rodal_pointer!([T: ?Sized] * mut T = *T);
rodal_pointer!([T] AtomicPtr<T> = *T);
rodal_object_reference!([T: ?Sized + Dump] Box<T> = &T);

rodal_value!(AtomicBool);
rodal_value!(AtomicIsize);
rodal_value!(AtomicUsize);

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

rodal_enum!([T: Dump] Option<T> {None, (Some: val)} = Option<T>);

////////////////////////////////////////////////////////////////////////////////
// Implemention for [T; N] where T: Dump
macro_rules! rodal___array_impls {
    ($($len:tt)+) => { $(rodal_array_impl!($len);)+ }
}

/// If you need arrays of larger sizes, just use array_impl(size)
rodal___array_impls!(01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16
             17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32);


/// Impls for Tuple types
macro_rules! rodal___tuple_impls {
    ($(($($n:tt $name:ident)+))+) => { $(rodal_tuple_impl!($($n $name)+);)+ }
}

/// If you need more types than these, just use tuple_impl directly
rodal___tuple_impls! {
    (0 T0)
    (0 T0 1 T1)
    (0 T0 1 T1 2 T2)
    (0 T0 1 T1 2 T2 3 T3)
    (0 T0 1 T1 2 T2 3 T3 4 T4)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14)
    (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15)
}


// These definitions are copied from the standard library
// This is neccesary so we can use private fields, and types
// that are unstable, by making copies whith identicall layouts
// But these prefixes are prefixed with Rodal

// Note: the types declared here but without 'pub' are either private to the real standard libarary
// or are unstable
mod fake_std {
    use rodal;
    use rodal::*;
    use std;
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
        pub fn as_ptr_mut(&mut self) ->&mut *mut T { unsafe{std::mem::transmute(self)} }
        pub fn as_ptr(&self) ->& *const T { unsafe{std::mem::transmute(self)} }
    }

    rodal_object_reference!([T: ?Sized + Dump] (Unique<T>) = &T);

    /// core::ptr (libcore/ptr.rs)
    struct Shared<T: ?Sized> { pub pointer: NonZero<*const T>, _marker: std::marker::PhantomData<T> }
    rodal_object_reference!([T: ?Sized + Dump] Shared<T> = &T);

    /// collections::vec (libcollections/vec.rs)
    pub struct Vec<T> { buf: RawVec<T>, pub len: usize }
    unsafe impl<T: Dump> Dump for std::vec::Vec<T> {
        fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D) {
            trace!("{:?}: Vec<T>::dump(dumper)", super::Address::new(self));

            // Transmute to the fake_std version so we can access private fields
            let fake_self: &Vec<T> = unsafe{std::mem::transmute(self)};

            dumper.reference_object_function_sized_position(
                self, // the argument to pass to the dump function
                // The function to use to dump the contents
                unsafe{std::mem::transmute::<fn(&Vec<T>, &mut D), DumpFunction<D>>(Vec::<T>::dump_contents)},
                *fake_self.buf.ptr.as_ref(), // Where to actually dump the data
                std::mem::size_of::<T>()*fake_self.buf.cap, std::mem::align_of::<T>());

            // Dump the fields of the vector
            dumper.dump_reference(fake_self.buf.ptr.as_ref());
            dumper.dump_object(&fake_self.buf.cap);
            dumper.dump_object(&fake_self.len);
        }
    }
    impl <T: Dump> Vec<T> {
        fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
            trace!("{:?}: Vec<T>::dump_contents(dumper)", super::Address::new(self));
            // Dump each element of the vector
            for i in 0..self.len {
                unsafe{(*self.buf.ptr.as_ptr().offset(i as isize)).dump(dumper)}
            }
        }
    }
    /// alloc::raw_vec (liballoc/rawvec.rs)
    struct RawVec<T> { pub ptr: Unique<T>, pub cap: usize }

    //collections::string (src/libcollections/string.rs)
    pub struct String { pub vec: std::vec::Vec<u8> }
    rodal_struct!([] std::string::String{vec} = String);

    // alloc::arc (liballoc/arc.rs)
    pub struct Arc<T: ?Sized> { ptr: Shared<ArcInner<T>>, }
    rodal_struct!([T: ?Sized + Dump] std::sync::Arc<T>{ptr} = Arc<T>);

    // alloc::arc (liballoc/arc.rs)
    struct ArcInner<T: ?Sized> { strong: std::sync::atomic::AtomicUsize, weak: std::sync::atomic::AtomicUsize, data: T, }
    rodal_struct!([T: ?Sized + Dump] ArcInner<T>{strong, weak, data});

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
            trace!("{:?}: RwLock<T>::dump", super::Address::new(self));
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
            trace!("{:?}: fake_std::sys::RWLock::dump", super::Address::new(self));

            // Create a new std::sync::RwLock, and dump its value of inner
            // (so that when we load the dump the RwLock will have it's initial state)
            let lock: RwLock<()> = unsafe{ std::mem::transmute(std::sync::RwLock::<()>::new(())) };
            dumper.dump_value_here(lock.inner.as_ref());
        }
    }

    // std::collections::hash_map (src/libstd/collections/hash/map.rs)
    pub struct RandomState { pub k0: u64, pub k1: u64, }
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
            trace!("{:?}: HashMap<K, V, S>::dump(dumper)", super::Address::new(self));
            // Transmute to the fake_std version so we can access private fields
            let fake_self: &HashMap<K, V, S> = unsafe{std::mem::transmute(self)};

            dumper.dump_object_here(&fake_self.hash_builder);

            // Dump table
            dumper.dump_object_here(&fake_self.table.capacity);
            dumper.dump_object_here(&fake_self.table.size);
            if fake_self.table.capacity == 0 {
                // Not an actual pointer (there is no associated memory)
                dumper.dump_value(&fake_self.table.hashes);
            } else {
                // Compute the size and alignment of the associated memory area
                // (this was adapted from the real std's RawTable's Drop function)
                let hash_size = fake_self.table.capacity*std::mem::size_of::<HashUint>();
                let hash_align = std::mem::align_of::<HashUint>();
                let pairs_size = fake_self.table.capacity*std::mem::size_of::<(K, V)>();
                let pairs_align = std::mem::align_of::<(K, V)>();

                // Rounds up hash_size to be a multiple of pairs_align
                // (this works as pairs_align is a power of 2)
                let hash_end = (hash_size + pairs_align - 1) & !(pairs_align - 1);

                let size = hash_end + pairs_size;
                let align = std::cmp::max(hash_align, pairs_align);

                // WARNING This won't save the tag of the pointer...
                dumper.reference_object_function_sized_position(
                    fake_self, // the argument to pass to the dump function
                    // The function to use to dump the contents
                    unsafe{std::mem::transmute::<fn(&HashMap<K, V, S>, &mut D), DumpFunction<D>>(
                        HashMap::<K, V, S>::dump_contents)},
                    unsafe{&*fake_self.table.hashes.ptr()}, // Where to actually dump the data
                    size, align);
                dumper.dump_object_here(&fake_self.table.hashes);
            }
            dumper.dump_object_here(&fake_self.resize_policy);
        }
    }
    impl<K: Eq + std::hash::Hash + Dump, V: Dump, S: std::hash::BuildHasher + Dump>
    HashMap<K, V, S> {
        fn dump_contents<D: ?Sized + Dumper>(&self, dumper: &mut D) {
            trace!("{:?}: HashMap<K, V, S>::dump_contents(dumper)", super::Address::new(self));
            // Dump the stored hashes
            dumper.dump_value_sized(unsafe{&*self.table.hashes.ptr()},
                    self.table.capacity*std::mem::size_of::<HashUint>());

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
        fn ptr(&self) -> *mut HashUint { (*self.0.as_ptr() as usize & !1) as *mut HashUint }
    }
    type HashUint = usize;
    struct RawTable<K, V> {
        pub capacity: usize,
        pub size: usize,
        pub hashes: TaggedHashUintPtr,
        pub marker: std::marker::PhantomData<(K, V)>,
    }
}