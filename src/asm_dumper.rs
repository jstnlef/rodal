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

use num::integer::lcm;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::io::Write;
use std::mem;
use super::*;

#[derive(Clone, Default)]
struct AsmLabel {
    base: String,
    offset: isize,
}

impl AsmLabel {
    fn new(name: String) -> AsmLabel {
        let name = if cfg!(target_os = "macos") {
            "_".to_string() + &name
        } else {
            name
        };
        AsmLabel { base: name, offset: 0 }
    }
    // Move the label by the specified number of bytes
    fn offset(&self, offset: isize) -> AsmLabel {
        AsmLabel {
            base: self.base.clone(),
            offset: self.offset + offset,
        }
    }
}

impl fmt::Display for AsmLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{:+}", self.base, self.offset)
    }
}

struct ObjectInfo<W: Write> {
    start: Address,
    size: usize,
    alignment: usize,
    label: AsmLabel,
    value: Address,
    // The arg to pass to dump
    dump: DumpFunction<AsmDumper<W>>,
}

impl<W: Write> Clone for ObjectInfo<W> {
    fn clone(&self) -> ObjectInfo<W> {
        ObjectInfo::<W> {
            start: self.start,
            size: self.size,
            alignment: self.alignment,
            label: self.label.clone(),
            value: self.value,
            dump: self.dump,
        }
    }
}

impl<W: Write> ObjectInfo<W> {
    fn new(
        value: Address,
        dump: DumpFunction<AsmDumper<W>>,
        start: Address,
        size: usize,
        alignment: usize,
        label: AsmLabel,
    ) -> ObjectInfo<W> {
        ObjectInfo {
            start: start,
            size: size,
            label: label,
            alignment: alignment,
            value: value,
            dump: dump,
        }
    }
}

#[cfg(target_arch = "x86_64")]
const POINTER_DIRECTIVE: &str = ".quad";
#[cfg(target_arch = "aarch64")]
const POINTER_DIRECTIVE: &str = ".xword";

enum AsmDirective {
    Byte,
    // Whe are inside a .byte
    Ptr,
    // We are inside a POINTER_DIRECTIVE
    Other,  // we aran't inside either
}

pub struct AsmDumper<W: Write> {
    file: W,
    current_directive: AsmDirective,

    current_pointer: Address,
    // This is the pointer into the output we are dumping
    position_offset: isize,   // The offset from current_pointer to tell objects where we are

    #[cfg(debug_assertions)]
    debug_stack: Vec<Address>,
    // For debugging only
    #[cfg(debug_assertions)]
    debug_indent: Vec<usize>, // How much to indent debugging info by

    /// Objects we've already dumped
    dumped_objects: HashMap<Address, ObjectInfo<W>>,

    /// A table of all objects we haven't started dumping yet
    pending_objects: HashMap<Address, ObjectInfo<W>>,

    /// Objects we're currently dumping
    dumping_objects: HashMap<Address, ObjectInfo<W>>,

    /// References that haven't been resolved to be relative to a complete object yet
    pending_references: BTreeSet<Address>,
    tags: HashMap<usize, Vec<*const ()>>,
}

impl<W: Write> AsmDumper<W> {
    #[cfg(debug_assertions)]
    pub fn new(mut file: W) -> AsmDumper<W> {
        writeln!(file, "#START RODAL DUMP").unwrap();
        writeln!(file, "\t.data").unwrap();
        AsmDumper::<W> {
            file: file,
            current_directive: AsmDirective::Other,
            current_pointer: Address::null(),
            position_offset: 0,
            debug_stack: Vec::new(),
            debug_indent: Vec::new(),
            dumped_objects: HashMap::new(),
            pending_objects: HashMap::new(),
            dumping_objects: HashMap::new(),
            pending_references: BTreeSet::new(),
            tags: HashMap::new(),
        }
    }
    #[cfg(not(debug_assertions))]
    pub fn new(mut file: W) -> AsmDumper<W> {
        writeln!(file, "#START RODAL DUMP").unwrap();
        writeln!(file, "\t.data").unwrap();
        AsmDumper::<W> {
            file: file,
            current_directive: AsmDirective::Other,
            current_pointer: Address::null(),
            position_offset: 0,
            dumped_objects: HashMap::new(),
            pending_objects: HashMap::new(),
            dumping_objects: HashMap::new(),
            pending_references: BTreeSet::new(),
            tags: HashMap::new(),
        }
    }
    pub fn dump_sized<T: ? Sized + Dump>(&mut self, name: &str, value: &T, size: usize, alignment: usize) -> &mut Self {
        assert!(alignment != 0);

        let start = Address::new(value);
        let label = AsmLabel::new(name.to_string());
        //trace!("{}: dump_sized({}, {}, {}, {})", self.current_pointer, name, start, size, alignment);

        self.current_pointer = Address::new(value);

        debug_only!({
            trace!("");
            trace!("dumping {} [{}, {:+}):", label.base.clone(), start, size)
        });
        self.write_global(&label);
        self.write_type_object(&label);
        self.write_size_align(size, alignment);
        self.write_label_declaration(&label);
        let dump_function = Self::get_dump_function::<T>();
        self.dumped_objects.insert(
            start,
            ObjectInfo::<W>::new(start, dump_function, start, size, alignment, label.clone()),
        );
        self.dump_object_function_here(value, dump_function);
        self.advance_position(start + size); // Add any neccesary padding
        self.write_size(&label);
        self
        // We finished dumping this root object
    }

    pub fn dump_tags(&mut self) {
        self.finish(); // Dump eveything that might need to be tagged

        // This is totally undefined bheaviour
        // as this creates a immutable borrow to self (the reference to self.tags)
        // and we then create a muttable borrow (to self in the call to self.dump)
        //let tags = Address::new(&self.tags);
        let tags: HashMap<usize, Vec<*const ()>> = self.tags.clone(); // This is soo unnecesary...
        self.dump("RODAL_TAGS", &tags);
    }
    pub fn finish(&mut self) {
        //trace!("{:?}: finish()", self.current_pointer);

        // Still more objects to dump
        while !self.pending_objects.is_empty() {
            //self.dumping_objects.append(&mut self.pending_objects);
            for (key, value) in self.pending_objects.drain() {
                self.dumping_objects.insert(key, value);
            }
            for (start, value) in self.dumping_objects.clone() {
                // Remove an element from the map (it dosn't mater which one)
                // And add a copy to dumped_objects
                self.current_pointer = start;

                debug_only!({
                    trace!("");
                    trace!("dumping {} [{}, {:+}):", value.label.base.clone(), start, value.size)
                });
                self.write_type_object(&value.label);
                self.write_size_align(value.size, value.alignment);
                self.write_label_declaration(&value.label);
                self.dump_object_function_here(value.value.to_ref::<()>(), value.dump);
                self.advance_position(start + value.size);
                self.write_size(&value.label);
            }
            for (key, value) in self.dumping_objects.drain() {
                self.dumped_objects.insert(key, value);
            }
            //self.dumped_objects.append(&mut self.dumping_objects);
        }

        assert!(self.pending_references.is_empty()); // We should've dumped all referenced objects by now

        // Write a label indicating the end of the rodal dump
        let end_label = AsmLabel::new("RODAL_END".to_string());
        self.write_global(&end_label);
        self.write_label_declaration(&end_label);

        writeln!(self.file, "#END RODAL DUMP").unwrap();
    }

    #[inline]
    fn start_directive(&mut self, new_directive: AsmDirective) {
        match self.current_directive {
            // End the directive with a newline
            AsmDirective::Byte | AsmDirective::Ptr => {
                writeln!(self.file).unwrap();
            }
            _ => {}
        }
        self.current_directive = new_directive;
    }
    #[inline]
    pub fn dump<T: ? Sized + Dump>(&mut self, name: &str, value: &T) -> &mut Self {
        self.dump_sized(name, value, mem::size_of_val(value), mem::align_of_val(value))
    }
    #[inline]
    fn write_skip(&mut self, size: usize) {
        self.start_directive(AsmDirective::Other);
        writeln!(self.file, "\t.skip {}", size).unwrap();
    }

    #[inline]
    fn write_byte(&mut self, value: u8) {
        match self.current_directive {
            // Continue the current byte directive
            AsmDirective::Byte => {
                write!(self.file, ", ").unwrap();
            }
            _ => {
                self.start_directive(AsmDirective::Byte);
                // Start a new byte directive
                write!(self.file, "\t.byte ").unwrap();
            }
        }

        write!(self.file, "{:#02x}", value).unwrap();
    }

    #[inline]
    fn write_size(&mut self, label: &AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        if cfg!(target_os = "linux") {
            // Not suported on macosx
            writeln!(self.file, "\t.size {}, .-{}", label.base, label.base).unwrap();
            writeln!(self.file).unwrap();
        }
    }

    #[inline]
    fn write_equiv(&mut self, target: AsmLabel, source: AsmLabel) {
        self.start_directive(AsmDirective::Other);
        writeln!(self.file, "\t.equiv {}, {}", target.base, source.offset(-target.offset)).unwrap();
    }

    #[inline]
    fn write_label_reference(&mut self, label: AsmLabel) {
        match self.current_directive {
            // Continue the current ptr directive
            AsmDirective::Ptr => write!(self.file, ", {}", label).unwrap(),
            _ => {
                self.start_directive(AsmDirective::Ptr);
                // Start a new ptr directive
                write!(self.file, "\t{} {}", POINTER_DIRECTIVE, label).unwrap();
            }
        }
    }

    #[inline]
    // We need to write the size of objects so that we can handle it if
    // realloc is called on one
    fn write_size_align(&mut self, size: usize, alignment: usize) {
        // We need to align to usize as we will store a usize indicating the size of the object
        let alignment = lcm(mem::align_of::<usize>(), alignment);
        self.start_directive(AsmDirective::Other);
        writeln!(self.file, "\t.balign {}", alignment).unwrap();

        // Add neccesary padding so that the data for the object is properly aligned
        let padding = alignment - mem::size_of::<usize>();
        if padding > 0 {
            write!(self.file, "\t.skip {}", padding).unwrap();
        }

        // Write the size, which will be aligned to mem::align_of::<usize>()
        writeln!(self.file, "\t{} {}", POINTER_DIRECTIVE, size).unwrap();

        // Now the next thing that is written will be aligned to alignment
        // And have a properly aligned usize immediatly before it
    }

    #[inline]
    fn write_global(&mut self, label: &AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        writeln!(self.file, "\t.globl {}", label.base).unwrap();
    }
    #[inline]
    fn write_type_object(&mut self, label: &AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        if cfg!(target_os = "linux") {
            // Not suported on macosx
            writeln!(self.file, "\t.type {}, %object", label.base).unwrap();
        }
    }
    #[inline]
    fn write_label_declaration(&mut self, label: &AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        writeln!(self.file, "{}:", label.base).unwrap();
    }

    #[inline]
    /// Advanced the current pointer to the specified address, adding padding as neccesary
    fn advance_position(&mut self, address: Address) {
        //trace!("{:?}: advance_position({:?})", self.current_pointer, address);

        let padding = address - self.current_pointer;
        assert!(
            padding >= 0,
            "can't advance from {} to {}",
            self.current_pointer,
            address
        );
        if padding != 0 {
            self.write_skip(padding as usize);
        }
        self.current_pointer += padding;
        //trace!("+ {} -> {:?}", padding, self.current_pointer);
    }

    #[inline]
    fn get_object(&mut self, start: Address, size: usize, alignment: usize) -> Option<AsmLabel> {
        let res = match &self.dumped_objects.get(&start) {
            &Some(value) => Some(value.label.clone()),
            &None => match &self.pending_objects.get(&start) {
                &Some(value) => Some(value.label.clone()),
                &None => match &self.dumping_objects.get(&start) {
                    &Some(value) => Some(value.label.clone()),
                    &None => None
                }
            }
        };

        // Check that objects don't overlap
        debug_only!({
            let object = if self.dumped_objects.contains_key(&start) {
                self.dumped_objects.get(&start)
            } else if self.pending_objects.contains_key(&start) {
                self.pending_objects.get(&start)
            } else if self.dumping_objects.contains_key(&start) {
                // self.dumping_objects.contains_key(&start)
                self.dumping_objects.get(&start)
            } else {
                None
            };

            if object.is_some() {
                let object = object.unwrap();
                debug_assert!(
                    object.size == size && object.alignment == alignment,
                    "conflicting layouts for object [{}], got size = {} and {}, and alignment = {} and {}",
                    start,
                    object.size,
                    size,
                    object.alignment,
                    alignment
                );
            }
        });

        res
    }

    #[inline]
    fn new_object(
        &mut self,
        start: Address,
        size: usize,
        alignment: usize,
        value: Address,
        dump: DumpFunction<Self>,
    ) -> AsmLabel {
        // This is the first time we've called reference_object on this pointer
        let label = AsmLabel::new(format!("object_{}", start));

        // For each overlaping pending reference, update it's label and delete it
        // We can't iterate over a collection and delete simultaneusly
        // Also the insane borrow checker won't let me call write_equiv within the loop either
        let mut delete_keys: Vec<Address> = Vec::new(); // A list of keys to delete from pending_references
        let mut write_equiv_args: Vec<(AsmLabel, AsmLabel)> = Vec::new();
        for ptr in self.pending_references.range(start..start + size) {
            // Any reference that overlaps with a complete object should be entirely contained by that object
            write_equiv_args.push((AsmLabel::new(format!(".Lptr_{}", ptr)), label.offset(*ptr - start)));
            delete_keys.push(*ptr);
        }
        for (source, target) in write_equiv_args {
            self.write_equiv(source, target);
        }
        for key in delete_keys {
            self.pending_references.remove(&key);
        }

        // Value is suposed to be a new complete object, so verify it does
        // not overlap with any other complete objects
        debug_only!({
            /*debug_assert!(get_overlap(start, start+size, &mut self.dumped_objects).count() == 0,
                "the object range [{}, {}) overlaps with a complete object", start, start+size);
            debug_assert!(get_overlap(start, start+size, &mut self.pending_objects).count() == 0,
                "the object range [{}, {}) overlaps with a complete object", start, start+size);
            debug_assert!(get_overlap(start, start+size, &mut self.dumping_objects).count() == 0,
                "the object range [{}, {}) overlaps with a complete object", start, start+size);*/
        });
        self.pending_objects.insert(
            start,
            ObjectInfo::new(value, dump, start, size, alignment, label.clone()),
        );
        label
    }
}

// WARNING: Never dump an object of zero size (i.e. such an object should have a trivial dump method)
impl<W: Write> Dumper for AsmDumper<W> {
    fn tag_reference<T: ? Sized>(&mut self, value: &T, tag: usize) {
        //trace!("{:?}: TAG_reference({:?}, {})", self.current_pointer, Address::new(value), tag);
        let value = Address::new(value).to_ptr::<()>();

        match &mut self.tags.get_mut(&tag) {
            &mut Some(ref mut vec) => {
                return vec.push(value);
            } // Add to the existing list
            &mut None => {}
        }
        self.tags.insert(tag, vec![value]); // Add a new list
    }
    /// Record the given complete object as needing to be dumped (because it is referenced)
    fn reference_object_function_sized_position<T: ? Sized, P: ? Sized>(
        &mut self,
        value: &T,
        dump: DumpFunction<Self>,
        position: &P,
        size: usize,
        alignment: usize,
    ) {
        // Objects with zero size should never be referenced
        // If they could be, then there could be ambiguouty if a complete object contains this address,
        // and we have a pointer with the value, does it point to this object of zero size, or the other overlaping one?
        // Or should we never allow pointers to them? Since there is technically no byte within the bounds of the object
        // yet the object has an address...
        // Just don't allow them, it makes things simpler.
        assert!(size != 0 && alignment != 0);
        let start = Address::new(position);
        debug_only!(trace!(
            "{empty:indent$} =>{location}",
            empty = "",
            indent = *self.debug_indent.last().unwrap(),
            location = start
        ));

        //trace!("{:?}: reference_object_sized_position({}, {}, {}, {})", self.current_pointer, Address::new(value), start, size, alignment);

        // If we don't have a record for this object
        if self.get_object(start, size, alignment).is_none() {
            self.new_object(start, size, alignment, Address::new(value), dump);
        }
    }

    /// Record the given complete object as needing to be dumped, and dump a reference to it
    fn dump_reference_object_function_sized_position_offset_here<T: ? Sized, P: ? Sized>(
        &mut self,
        value: &T,
        dump: DumpFunction<Self>,
        position: &&P,
        size: usize,
        alignment: usize,
        offset: isize,
    ) {
        assert!(size != 0 && alignment != 0);
        let start = Address::new(*position);
        debug_only!(trace!(
            "{empty:indent$} -=>{location}",
            empty = "",
            indent = *self.debug_indent.last().unwrap(),
            location = start
        ));

        //trace!("{:?}: dump_reference_object_sized_position({}, {}, {}, {})", self.current_pointer, Address::new(value), start, size, alignment);

        let label = self.get_object(start, size, alignment);

        // We don't
        let label = match label {
            Some(label) => label,
            None => self.new_object(start, size, alignment, Address::new(value), dump)
        };

        // Write the label
        self.write_label_reference(label.offset(offset));
        self.current_pointer += mem::size_of::<&&P>();
    }

    fn dump_reference_here<T: ? Sized>(&mut self, value: &&T) {
        let ptr = Address::new(*value);
        debug_only!(trace!(
            "{empty:indent$} ->{location}",
            empty = "",
            indent = *self.debug_indent.last().unwrap(),
            location = ptr
        ));

        //trace!("{:?}: dump_reference_here({:?} = &{})", self.current_pointer, Address::new(value), ptr);

        // Look for a recorded complete object containg this,..
        let label = match get_complete_object(ptr, &self.dumped_objects) {
            Some(value) => value.label.offset(ptr - value.start),
            None => match get_complete_object(ptr, &self.pending_objects) {
                Some(value) => value.label.offset(ptr - value.start),
                None => match get_complete_object(ptr, &self.dumping_objects) {
                    Some(value) => value.label.offset(ptr - value.start),
                    None => {
                        // Just create a temporary label, and record our pointer
                        self.pending_references.insert(ptr);
                        AsmLabel::new(format!(".Lptr_{}", ptr))
                    }
                }
            }
        };

        self.write_label_reference(label);
        // Pointerr & references should always have the same size, so theres no need to override this
        self.current_pointer += mem::size_of::<&&T>();
        //trace!("+ {} -> {:?}", mem::size_of::<&&T>(), self.current_pointer);
    }

    /// Dump the raw value of the object
    fn dump_value_sized_here<T: ? Sized>(&mut self, value: &T, size: usize) {
        let value = Address::new(value);
        //trace!("{:?}: dump_value_sized_here({:?}, {})", self.current_pointer, value, size);

        for i in 0..size {
            // Write the byte
            self.write_byte(*(value + i).to_ref::<u8>());
        }
        self.current_pointer += size;
        //trace!("+ {} -> {:?}", size, self.current_pointer);
    }

    fn dump_padding_sized(&mut self, size: usize) {
        //trace!("{:?}: dump_padding_sized({})", self.current_pointer, size);

        if size != 0 {
            self.write_skip(size);
        }
        self.current_pointer += size;
        //trace!("+ {} -> {:?}", size, self.current_pointer);
    }

    #[cfg(debug_assertions)]
    fn debug_record<T: ? Sized + Named>(&mut self, func_name: &str) {
        // Print the level, followed by a collen, followed 'level' spaces,
        // followed by the offset (with a sign) a tab, and the type and function name
        let indent = format!(
            "{level}:{empty:level$}{offset:+}:        ",
            level = self.debug_stack.len(),
            empty = "",
            offset = self.current_pointer - *self.debug_stack.last().unwrap_or(&self.current_pointer)
        );
        self.debug_indent.push(indent.len());

        trace!(
            "{}{type_name}::{func_name}",
            indent,
            type_name = T::name(),
            func_name = func_name
        );
        self.debug_stack.push(self.current_pointer);
    }

    // Set the current position to be returned by current_position
    // (dosn't actually effect the internal pointer, used to tell other objects where they should think they are)
    fn set_position(&mut self, new_position: Address) {
        self.position_offset = new_position - self.current_pointer;
        assert!(self.current_position() == new_position);
    }
    fn current_position(&self) -> Address {
        self.current_pointer + self.position_offset
    }
    // CALL This whenever we execute a dump function...
    fn dump_object_function_here<T: ? Sized>(&mut self, value: &T, dump: DumpFunction<Self>) {
        let old_offset = self.position_offset;
        let value = Address::new(value);
        self.set_position(value);

        //trace!("{:?}: dump_object_function_here({:?}, {})", self.current_pointer, Address::new(value), unsafe{mem::transmute::<DumpFunction<Self>, Address>(dump)});
        (dump)(value.to_ref::<()>(), self);
        debug_only!({
            self.debug_stack.pop();
            self.debug_indent.pop()
        });
        //debug_only!();
        self.position_offset = old_offset;
    }
}

// Returns the range of elements in the map that overlaps with [start, end)
// Only used to check invariants in debug mode
/*#[cfg(debug_assertions)]
fn get_overlap<'a, W: Write>(start: Address, end: Address, map: &'a mut HashMap<Address, ObjectInfo<W>>)
                      -> RangeMut<'a, Address, ObjectInfo<W>> {

    let start = match map.range_mut(..start).last() {
        Some((key, value)) =>
            if value.start + value.size > start { *key }
                else { start },
        _ => start
    };
    map.range_mut(start..end)
}*/

// Gets the complete object that contains start
fn get_complete_object<'a, W: Write>(_: Address, _: &'a HashMap<Address, ObjectInfo<W>>) -> Option<&'a ObjectInfo<W>> {
    unimplemented!();
    /*match map.range((Bound::Unbounded, Bound::Included(start))).last() {
        Some((_, value)) => {
            if value.start + value.size > start {
                Some(value)
            } else {
                None
            }
        }
        None => None
    }*/
}
