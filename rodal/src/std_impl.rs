/// Implements Dump and Load for various standard library types
use Dump;
use Dumper;

use std::{cmp, iter, mem, ops, slice, str};
use std::{i8, i16, i32, i64, isize};
use std::{u8, u16, u32, u64, usize};
use std::{f32, f64};

use std::cell::{Cell, RefCell};
use std::clone::{self, Clone};
use std::convert::{self, From, Into};
use std::default::{self, Default};
use std::fmt::{self, Debug, Display};
use std::marker::{self, PhantomData};
use std::option::{self, Option};
use std::result::{self, Result};
use std::borrow::{Cow, ToOwned};
use std::string::String;
use std::vec::Vec;
use std::boxed::Box;
use std::rc::Rc;
use std::sync::Arc;
use std::collections::{BinaryHeap, BTreeMap, BTreeSet, LinkedList, VecDeque};
use std::{error, net};
use std::collections::{HashMap, HashSet};
use std::ffi::{CString, CStr, OsString, OsStr};
use std::hash::{Hash, BuildHasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::{Mutex, RwLock};
use std::ptr::{Shared, Unique};

// Dump implementations for standard library types
macro_rules! value_impl {
    ($ty:ident) => {
        unsafe impl Dump for $ty {
            #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
            {
                dumper.dump_value(self)
            }
        }
    }
}

macro_rules! object_reference_impl {
    ($t: ident, $ty:ty) => {
        unsafe impl<$t> Dump for $ty where $t: Dump {
            #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
            {
                dumper.dump_reference_object(unsafe{mem::transmute::<&Self, &&$t>(self)});
            }
        }
    }
}
macro_rules! pointer_impl {
    ($t: ident, $ty:ty) => {
        unsafe impl<$t> Dump for $ty where $t: Dump {
            #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
            {
                dumper.dump_reference(unsafe{mem::transmute::<&Self, &&$t>(self)});
            }
        }
    }
}
macro_rules! reference_impl {
    ($t: ident, $ty:ty) => {
        unsafe impl<'a, $t> Dump for $ty where $t: Dump {
            #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
            {
                dumper.dump_reference(unsafe{mem::transmute::<&Self, &&$t>(self)});
            }
        }
    }
}

object_reference_impl!(T, Unique<T>);
object_reference_impl!(T, Shared<T>);
reference_impl!(T, &'a T);
reference_impl!(T, &'a mut T);
pointer_impl!(T, *const T);
pointer_impl!(T, *mut T);


value_impl!(bool);
value_impl!(isize);
value_impl!(i8);
value_impl!(i16);
value_impl!(i32);
value_impl!(i64);
value_impl!(usize);
value_impl!(u8);
value_impl!(u16);
value_impl!(u32);
value_impl!(u64);
value_impl!(f32);
value_impl!(f64);
value_impl!(char);

/*
STD Types we probably want to support:
(just add each type as we need them...)
(Unfortuantly we can't use the derive macros...)
str // Where is this defined
struct String { vec: Vec<u8> }

Option<T>
PhantomData<T> // Tricial
Maybye:
CStr
Cstring
[T]
BinaryHeap<T: Ord>
BTreeSet<T: Ord>
//HashSet<T: Eq + Hash, H: BuildHasher>
//LinkedList<T>
//Vec<T>
//VecDeque<T>
//ops::Range<Idx>
// ()
// Tuples...
// BTreeMap<K: Ord, V>
// HashMap<K: Eq + Hash, V, H: BuildHasher>
// References
//  &'a T;
//  &'a mut T
// Box<T>
// Rc<T>
// Arc<T>
// Cow<T>
// Wtf?
// NonZero<T>
// Cell<T>
//RefCell<T>
// Mutex<T>
// RwLock<T>
/// Result<T, E>
// Duration

*/


////////////////////////////////////////////////////////////////////////////////

/// Implemention for [T; N] where T: Dump
macro_rules! array_impls {
    ($($len:tt)+) => {
        $(
            unsafe impl<T> Dump for [T; $len] where T: Dump
            {
                #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
                {
                    for i in 0..$len {
                        dumper.dump_object(&self[i]);
                    }
                }
            }
        )+
    }
}

/// If you need arrays of larger sizes, just add more numbers to this
array_impls!(01 02 03 04 05 06 07 08 09 10
             11 12 13 14 15 16 17 18 19 20
             21 22 23 24 25 26 27 28 29 30
             31 32);


/// Impls for Tuple types
macro_rules! tuple_impls {
    ($($len:expr => ($($n:tt $name:ident)+))+) => {
        $(
            unsafe impl<$($name),+> Dump for ($($name,)+) where $($name: Dump,)+
            {
                #[inline] fn dump<D>(&self, dumper: &mut D) where D: Dumper
                {
                    $(
                        dumper.dump_object(&self.$n);
                    )+
                }
            }
        )+
    }
}

/// If you need more types than these, just add more lines
tuple_impls! {
    1 => (0 T0)
    2 => (0 T0 1 T1)
    3 => (0 T0 1 T1 2 T2)
    4 => (0 T0 1 T1 2 T2 3 T3)
    5 => (0 T0 1 T1 2 T2 3 T3 4 T4)
    6 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5)
    7 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6)
    8 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7)
    9 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8)
    10 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9)
    11 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10)
    12 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11)
    13 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12)
    14 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13)
    15 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14)
    16 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15)
}