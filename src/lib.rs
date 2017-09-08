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

#![feature(allocator_api)]
#![feature(alloc)]
#![feature(alloc_system)]
#![feature(global_allocator)]
#![feature(const_fn)]

extern crate libc;
extern crate num;

#[macro_use]
#[cfg(debug_assertions)]
extern crate log;
#[macro_use]
extern crate field_offset;

#[macro_use]
mod macros;
mod asm_dumper;
mod asm_loader;
mod alloc;
mod address;
mod rust_std;
mod extended_std;
pub use asm_dumper::*;
pub use asm_loader::*;
pub use alloc::*;
pub use address::*;
pub use extended_std::*;

pub trait Named {
    fn name()->String;
}
pub fn type_name<T: ?Sized + Named>()->String { T::name() }

pub unsafe trait Dump: Named {
    /// Dump this object into the given RODAL Dumper
    /// WARNING: this function should only ever be called by a Dumper
    /// (use dump_object if you want to dump an object whilst dumping another one
    /// or use the Dumper's provided methods to start a dump)
    fn dump<D: ?Sized + Dumper>(&self, dumper: &mut D);
}

use std::mem;
use std::collections::BTreeMap;

#[inline] fn as_void_ref<T: ?Sized>(value: &T)->&() {
    unsafe{mem::transmute(value as *const T as *const ())}
}
pub type DumpFunction<D: Dumper> = fn(&(), &mut D);
pub trait Dumper {
    // For debugging purposes, records that we are in the dump function 'func_name'
    // for the type 'type_name'
    #[cfg(debug_assertions)]
    fn debug_record<T: ?Sized + Named>(&mut self, func_name: &str);
    #[cfg(not(debug_assertions))]
    #[inline(always)] fn debug_record<T: ?Sized + Named>(&mut self, _: &str) { }

    fn set_position(&mut self, new_position: Address);
    /// Returns the address of the end of the last thing the dumper dumped
    fn current_position(&self) -> Address;

    fn dump_padding_sized(&mut self, size: usize);
    #[inline] fn dump_padding<T: ?Sized>(&mut self, target: &T) {
        let current = self.current_position();
        let target = Address::new(target);
        assert!(target >= current, "cant move backwards from {} to {}", current, target);
        self.dump_padding_sized((target - current) as usize);
    }

    fn dump_value_sized_here<T: ?Sized>(&mut self, value: &T, size: usize); // Core function
    #[inline] fn dump_value_sized<T: ?Sized>(&mut self, value: &T, size: usize) {
        self.dump_padding(value);
        self.dump_value_sized_here(value, size);
    }
    #[inline] fn dump_value_here<T: ?Sized>(&mut self, value: &T) {
        self.dump_value_sized_here(value, mem::size_of_val(value));
    }
    #[inline] fn dump_value<T: ?Sized>(&mut self, value: &T) {
        self.dump_padding(value);
        self.dump_value_sized_here(value, mem::size_of_val(value));
    }

    // Gives the reference a tag...
    fn tag_reference<T: ?Sized>(&mut self, value: &T, tag: usize);
    // Gives the current position a tag
    fn tag(&mut self, tag: usize) {
        let value = self.current_position().to_ref::<()>();
        self.tag_reference::<()>(value, tag);
    }

    // Dump the object with the specified function
    fn dump_object_function_here<T: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>); // Core function
    #[inline] fn dump_object_function<T: ?Sized + Dump>(&mut self, value: &T, dump: DumpFunction<Self>) {
        self.dump_padding(value);
        self.dump_object_function_here(as_void_ref(value), dump);
    }

    #[inline] fn dump_object_here<T: ?Sized + Dump>(&mut self, value: &T) {
        self.dump_object_function_here(as_void_ref(value), Self::get_dump_function::<T>());
    }
    #[inline] fn dump_object<T: ?Sized + Dump>(&mut self, value: &T) {
        self.dump_padding(value);
        self.dump_object_here(value);
    }

    fn dump_reference_here<T: ?Sized>(&mut self, value: &&T);
    #[inline] fn dump_reference<T: ?Sized>(&mut self, value: &&T) {
        self.dump_padding(value);
        self.dump_reference_here(value);
    }

    fn reference_object_function_sized_position<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &P, size: usize, alignment: usize);
    #[inline] fn reference_object_sized_position<T: ?Sized + Dump, P: ?Sized>(&mut self, value: &T, position: &P, size: usize, alignment: usize) {
        self.reference_object_function_sized_position(value, Self::get_dump_function::<T>(), position, size, alignment);
    }
    #[inline] fn reference_object_sized<T: ?Sized + Dump>(&mut self, value: &T, size: usize, alignment: usize) {
        self.reference_object_sized_position(value, value, size, alignment)
    }
    #[inline] fn reference_object<T: ?Sized + Dump>(&mut self, value: &T) {
        self.reference_object_sized(value, mem::size_of_val(value), mem::align_of_val(value))
    }

    fn dump_reference_object_function_sized_position_offset_here<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &&P, size: usize, alignment: usize, offset: isize);
    #[inline] fn dump_reference_object_function_sized_position_here<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &&P, size: usize, alignment: usize) { self.dump_reference_object_function_sized_position_offset_here(value, dump, position, size, alignment, 0)}
    #[inline] fn dump_reference_object_sized_position_here<T: ?Sized + Dump, P: ?Sized>(&mut self, value: &T, position: &&P, size: usize, alignment: usize) {
        self.dump_reference_object_function_sized_position_here(value, Self::get_dump_function::<T>(), position, size, alignment);
    }
    #[inline] fn dump_reference_object_sized_here<T: ?Sized + Dump>(&mut self, value: &&T, size: usize, alignment: usize) {
        self.dump_reference_object_sized_position_here(*value, value, size, alignment)
    }
    #[inline] fn dump_reference_object_here<T: ?Sized + Dump>(&mut self, value: &&T) {
        self.dump_reference_object_sized_here(value, mem::size_of_val(*value), mem::align_of_val(value))
    }

    #[inline] fn dump_reference_object_function_sized_position_offset<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &&P, size: usize, alignment: usize, offset: isize) {
        self.dump_padding(position);
        self.dump_reference_object_function_sized_position_offset_here(value, dump, position, size, alignment, offset);
    }
    #[inline] fn dump_reference_object_function_sized_position<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &&P, size: usize, alignment: usize) {
        self.dump_padding(position);
        self.dump_reference_object_function_sized_position_here(value, dump, position, size, alignment);
    }
    #[inline] fn dump_reference_object_sized_position<T: ?Sized + Dump, P: ?Sized>(&mut self, value: &T, position: &&P, size: usize, alignment: usize) {
        self.dump_padding(position);
        self.dump_reference_object_sized_position_here(value, position, size, alignment);
    }
    #[inline] fn dump_reference_object_sized<T: ?Sized + Dump>(&mut self, value: &&T, size: usize, alignment: usize) {
        self.dump_padding(value);
        self.dump_reference_object_sized_here(value, size, alignment)
    }
    #[inline] fn dump_reference_object<T: ?Sized + Dump>(&mut self, value: &&T) {
        self.dump_padding(value);
        self.dump_reference_object_here(value)
    }

    // For dumping enums
    // (since the discriminant is a raw value and needs to be stored, but it may be at the begining or end of the enum)
    #[inline] fn dump_prefix_value_here<T: ?Sized, U: ?Sized>(&mut self, start: &T, end: &U) {
        let distance = Address::new(end) - Address::new(start);
        assert!(distance >= 0, "prefix ends at {} before it starts {}", Address::new(end), Address::new(start));
        self.dump_value_sized_here(start, distance as usize);
    }
    #[inline] fn dump_prefix_value<T: ?Sized>(&mut self, end: &T) {
        let distance = Address::new(end) - self.current_position();
        assert!(distance >= 0, "prefix ends at {} before it starts {}", Address::new(end), self.current_position());
        let start = self.current_position().to_ref::<()>();
        self.dump_value_sized_here(start, distance as usize);
    }
    #[inline] fn dump_suffix_value_sized<T: ?Sized>(&mut self, start: &T, size: usize) {
        let distance = self.current_position() - Address::new(start);
        let end = self.current_position().to_ref::<()>();
        assert!(distance >= 0, "suffix starts at {} after the current position {}", Address::new(start), self.current_position());
        self.dump_value_sized_here(end, size - distance as usize);
    }
    #[inline] fn dump_suffix_value<T>(&mut self, start: &T) {
        self.dump_suffix_value_sized(start, mem::size_of::<T>())
    }

    #[inline] fn get_dump_function<T: ?Sized + Dump>()->DumpFunction<Self> {
        unsafe{mem::transmute::<fn(&T, &mut Self), DumpFunction<Self>>(T::dump::<Self>)}
    }
}

// For dumping parts of an object in arbitrary ordered (for use when rust reorders fields)
pub struct DumpList<D: ?Sized + Dumper> (BTreeMap<Address, (Address, DumpFunction<D>)>);
impl<D: ?Sized + Dumper> DumpList<D> {
    #[inline] pub fn new() -> DumpList<D> { DumpList::<D>(BTreeMap::new()) }
    #[inline] pub fn add_position<P: ?Sized, T: ?Sized + Dump>(&mut self, position: &P, value: &T) {
        if mem::size_of_val(position) != 0 { // Ignore zero sized types
            self.0.insert(Address::new(position), (Address::new(value), D::get_dump_function::<T>()));
        }
    }
    #[inline] pub fn add<T: ?Sized + Dump>(&mut self, value: &T) { self.add_position(value, value); }
    #[inline] pub fn dump(&mut self, dumper: &mut D) {
        for (position, &(value, dump)) in &self.0 {
            dumper.dump_padding(position.to_ref::<()>());
            dumper.dump_object_function_here(value.to_ref::<()>(), dump)
        }
        self.0.clear();
    }
    #[inline] pub fn first(&self)->&() { self.0.keys().next().unwrap().to_ref() }
}
