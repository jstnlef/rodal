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

// This files defines types that can be used to dump instances of rust_std types
// without actually making the realy std types

use std;
use super::*;

// Use new() to make a new fake arc and you can dump that
// You can then reload a pointer to it as a real Arc
pub struct FakeArc<'a, T: 'a> {
    inner: &'a T
}

rodal_named!(['a, T: Dump] FakeArc<'a, T> [type_name!("rodal::FakeArc<{}>", T)]);
impl<'a, T: Dump> FakeArc<'a, T> {
    pub fn new(val: &'a T) -> FakeArc<'a, T> {
        FakeArc::<'a, T> { inner: val }
    }

    // Dump an ArcInner
    fn dump_inner<D: ? Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump_inner");

        let start = dumper.current_position();
        // Dump the default strong value of 1
        dumper.dump_padding((start + offset_of!(rust_std::ArcInner<T> => strong).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(&std::sync::atomic::AtomicUsize::new(1));

        // Dump the default weak value of 1
        dumper.dump_padding((start + offset_of!(rust_std::ArcInner<T> => weak).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(&std::sync::atomic::AtomicUsize::new(1));

        // Dump the containing data
        dumper.dump_padding((start + offset_of!(rust_std::ArcInner<T> => data).get_byte_offset()).to_ref::<()>());
        dumper.dump_object_here(self.inner);
    }
}

unsafe impl<'a, T: Dump> Dump for FakeArc<'a, T> {
    fn dump<D: ? Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");

        // Where to dump our fake ArcInner
        // (make it include the real position of inner so that references to inside of inner will be correctly preserved)
        let fake_inner = (Address::new(self.inner) - offset_of!(rust_std::ArcInner<T> => data).get_byte_offset())
            .to_ref::<rust_std::ArcInner<T>>();

        dumper.dump_padding(&self.inner);
        dumper.dump_reference_object_function_sized_position_here(
            self, // the argument to pass to the dump function
            // The function to use to dump the contents
            unsafe {
                std::mem::transmute::<fn(&FakeArc<'a, T>, &mut D), DumpFunction<D>>(FakeArc::<'a, T>::dump_inner)
            },
            &fake_inner,
            std::mem::size_of::<rust_std::ArcInner<T>>(),
            std::mem::align_of::<rust_std::ArcInner<T>>(),
        );
    }
}

pub struct EmptyHashMap<K, V, S = std::collections::hash_map::RandomState>(rust_std::HashMap<K, V, S>);

impl<K: Eq + std::hash::Hash, V> EmptyHashMap<K, V, std::collections::hash_map::RandomState> {
    pub fn new() -> Self {
        unsafe {
            std::mem::transmute(std::collections::HashMap::<K, V, std::collections::hash_map::RandomState>::new())
        }
    }
}

// TODO: Rustc complains with 'transmute called with differently sized types: std::collections::HashMap<K, V, S> (size can vary because of S) to dump_std::EmptyHashMap<K, V, S> (size can vary because of S)'
/*impl<K: Eq + std::hash::Hash, V, S: std::hash::BuildHasher> EmptyHashMap<K, V, S> {
    fn with_hasher(hash_builder: S) -> Self { unsafe {std::mem::transmute(std::collections::HashMap::<K, V, S>::with_hasher(hash_builder))} }
}*/

// The Eq + Hash and BuildHasher contstratins are needed
// as almost all of the hashmap's code requires this
// (without them, we won't even be able to iterate over it's elements)
rodal_named!([K: Eq + std::hash::Hash + Named, V: Named, S: std::hash::BuildHasher + Dump] EmptyHashMap<K, V, S> [type_name!("rodal::EmptyHashMap<{}, {}, {}>", K, V, S)]);
unsafe impl<K: Eq + std::hash::Hash + Named, V: Named, S: std::hash::BuildHasher + Dump> Dump
for EmptyHashMap<K, V, S>
{
    fn dump<D: ? Sized + Dumper>(&self, dumper: &mut D) {
        dumper.debug_record::<Self>("dump");
        dumper.dump_object(&self.0.hash_builder);

        // Dump table
        dumper.dump_object(&self.0.table.capacity_mask);
        dumper.dump_object(&self.0.table.size);
        assert!(self.0.table.capacity() == 0);
        // Not an actual pointer (there is no associated memory)
        dumper.dump_value(&self.0.table.hashes);
        dumper.dump_object(&self.0.resize_policy);
    }
}

pub struct EmptyLinkedList<T>(rust_std::LinkedList<T>);

impl<T> EmptyLinkedList<T> {
    pub fn new() -> Self {
        unsafe { std::mem::transmute(std::collections::LinkedList::<T>::new()) }
    }
}
rodal_value!([T: Named] EmptyLinkedList<T> [type_name!("rodal::EmptyLinkedList<{}>", T)]);

pub struct EmptyOption<T>(Option<T>);

impl<T> EmptyOption<T> {
    pub fn new() -> Self {
        EmptyOption::<T>(None)
    }
}
rodal_value!([T: Named] EmptyOption<T> [type_name!("rodal::EmptyOption<{}>", T)]);
