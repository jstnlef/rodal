extern crate rodal;

use std::collections::BTreeMap;
use std::io::Write;
//use std::vec;
//use std::ops;
use std::fmt;
use std::cmp::max;
use std::collections::btree_map::RangeMut;
use std::mem;
use std::collections::Bound;
use rodal::*;

// Checks that the

// A utility trait that checks
trait CheckedCast<T> {
    fn checked_cast(self) -> T;
}
impl CheckedCast<isize> for usize {
    fn checked_cast(self) -> isize {
        assert!(self <= std::isize::MAX as usize);
        self as isize
    }
}
impl CheckedCast<usize> for isize {
    fn checked_cast(self) -> usize {
        assert!(self >= 0);
        self as usize
    }
}

enum AsmDirective {
    // Use other for a directive that can't be continued
    Byte, Ptr, Other // Ptr is a pseudo-directive it will translate to the appropriate directive for the target
}

// Handles pointer operations for us
#[derive(Eq, Default, Hash, PartialOrd, Clone, Copy, PartialEq, Ord, Debug)]
struct Address(usize);
impl Address {
    fn new<T>(value: &T) -> Address { Address(value as *const T as usize) }
    fn load<'a, T>(&self) -> &'a T { unsafe{mem::transmute::<*const T, &T>(self.0 as *const T)} }
    fn value(&self) -> usize { self.0 }
}
impl std::ops::Add<usize> for Address {
    type Output = Address;
    fn add(self, other: usize) -> Address { Address(self.0 + other) }
}
impl std::ops::Add<isize> for Address {
    type Output = Address;
    fn add(self, other: isize) -> Address {
        // This is neccesary to make rust perform overflow checking approprietly
        if other >= 0 { Address(self.0 + other as usize) }
            else { Address(self.0 - other.wrapping_neg() as usize) }
    }
}
impl std::ops::AddAssign<usize> for Address {
    fn add_assign(&mut self, other: usize) { self.0 += other }
}
impl std::ops::AddAssign<isize> for Address {
    fn add_assign(&mut self, other: isize) {
        if other >= 0 { self.0 += other as usize }
            else          { self.0 -= other.wrapping_neg() as usize }
    }
}
impl std::ops::Sub<usize> for Address {
    type Output = Address;
    fn sub(self, other: usize) -> Address { Address(self.0 - other) }
}
impl std::ops::Sub<isize> for Address {
    type Output = Address;
    fn sub(self, other: isize) -> Address {
        if other >= 0 { Address(self.0 - other as usize) }
            else          { Address(self.0 + other.wrapping_neg() as usize) }
    }
}
impl std::ops::SubAssign<usize> for Address {
    fn sub_assign(&mut self, other: usize) { self.0 -= other }
}
impl std::ops::SubAssign<isize> for Address {
    fn sub_assign(&mut self, other: isize) {
        if other >= 0 { self.0 -= other as usize }
            else          { self.0 += other.wrapping_neg() as usize }
    }
}
impl std::ops::Sub<Address> for Address {
    type Output = isize;
    fn sub(self, other: Address) -> isize {
        if self.0 >= other.0 {
            (self.0 - other.0).checked_cast()
        } else {
            let res = other.0 - self.0; // Will be positive
            assert!(res <= std::isize::MIN.wrapping_neg() as usize);
            (res as isize).wrapping_neg()
        }
    }
}
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl fmt::LowerHex for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}
#[derive(Clone, Default)]
struct AsmLabel {
    base: String,
    offset: isize,
}

impl AsmLabel {
    fn new_global(name: String) -> AsmLabel {
        AsmLabel {
            base: name,
            offset: 0
        }
    }
    // Creates a new label, to a complete object
    fn new_complete(id: usize) -> AsmLabel {
        AsmLabel {
            base: format!("object_{:#016x}", id),
            offset: 0
        }
    }
    // Creates a new label to inside of a data structure
    fn new_internal(id: usize) -> AsmLabel {
        AsmLabel {
            base: format!(".Lptr_{:#016x}", id),
            offset: 0
        }
    }

    // Move the label by the specified number of bytes
    fn offset(&self, offset: isize) -> AsmLabel {
        AsmLabel {base: self.base.clone(), offset: self.offset + offset}
    }
}

impl fmt::Display for AsmLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{:+}", self.base, self.offset)
    }
}

type DumpFunction<W> where W: Write = fn(&(), &mut AsmDumper<W>);
trait AddressRange {
    fn start(&self) -> Address;
    fn end(&self) -> Address;
    fn size(&self) -> usize;
}

struct ObjectInfo<W> where W: Write {
    start: Address,
    size: usize,
    alignment: usize,
    label: AsmLabel,
    //(&random_ref, &mut AsmDumper<W>)
    dump: DumpFunction<W>, //TODO: Rust might not like this...
}
impl<W> Clone for ObjectInfo<W> where W: Write {
    fn clone(&self) -> ObjectInfo<W> {
        ObjectInfo::<W> {
            start: self.start,
            size: self.size,
            alignment: self.alignment,
            label: self.label.clone(),
            dump: self.dump,
        }
    }
}
    fn start(&self)  -> Address { self.start }
    fn end(&self) -> Address { self.start + self.size}
    fn size(&self) -> usize { self.size }
impl<W> ObjectInfo<W> where W: Write {
    fn new<T>(value: &T, size: usize, alignment: usize, label: AsmLabel) -> ObjectInfo<W> where T: Dump {
        let address = Address::new(value);
        ObjectInfo {
            start: address,
            size: size,
            label: label,
            alignment: alignment,
            //fn(&T, &mut _) {<T as rodal::Dump>::dump::<_>}
            dump: unsafe{mem::transmute::<fn(&T, &mut AsmDumper<W>), DumpFunction<W>>(T::dump::<AsmDumper<W>>)},
        }
    }
}

/*#[derive(Clone)]
struct ReferenceInfo {
    start: Address,
    size: usize,
    label: AsmLabel,
}
impl AddressRange for ReferenceInfo {
    fn start(&self)  -> Address { self.start }
    fn end(&self) -> Address { self.start + self.size}
    fn size(&self) -> usize { self.size }
}

impl ReferenceInfo {
    fn new(value: Address, size: usize, label: &AsmLabel) -> ReferenceInfo {
        ReferenceInfo {
            start: value,
            size: size,
            label: label.clone(),
        }

    }
    fn new_internal<T>(value: &T, size: usize) -> ReferenceInfo {
        let address = Address::new(value);
        ReferenceInfo {
            start: address,
            size: size,
            label: AsmLabel::new_internal(address.value()),
        }
    }
}*/

pub struct AsmDumper<W> where W: Write
{
    file: W,
    current_pointer: Address,
    current_directive: AsmDirective,

    /// Objects we've already dumped (or started to dump)
    dumped_objects: BTreeMap<Address, ObjectInfo<W>>,

    /// A table of all objects we haven't started dumping yet
    pending_objects: BTreeMap<Address, ObjectInfo<W>>,

    /// References that haven't been resolved to be relative to a complete object yet
    pending_references: BTreeSet<Address>,
}

impl<W> AsmDumper<W> where W: Write {
    pub fn new(file: W) -> AsmDumper<W> {
        AsmDumper::<W> {
            file: file,
            current_pointer: Address::new(),
            current_directive: AsmDirective::Other,
            dumped_objects: BTreeMap::new(),
            pending_objects: BTreeMap::new(),
            pending_references: BTreeSet::new()
        }
    }
    pub fn dump_sized<T>(&mut self, name: String, value: &T, size: usize, alignment: usize) where T: Dump {
        assert!(alignment != 0);

        self.file.write_fmt(format_args!("#START DUMP OF {}\n", name)).unwrap();

        let start = Address::new(value);
        let label = AsmLabel::new_global(name.clone());
        self.current_pointer = Address::new(value);

        self.write_align(alignment);
        self.write_global(label.clone());
        self.write_label_declaration(label.clone());
        self.dumped_objects.insert(start, ObjectInfo::<W>::new(value, size, alignment, label.clone()));
        value.dump::<Self>(self);
        self.advance_position(start + size); // Add any neccesary padding
        self.write_size(label);
        self.file.write_all("\n".as_bytes()).unwrap();
        // We finished dumping the root object

        // Still more objects to dump
        while !self.pending_objects.is_empty() {
            // Remove an element from the map (it dosn't mater which one)
            // And add a copy to dumped_objects
            let key = self.pending_objects.keys().next().unwrap().clone();
            let value = self.pending_objects.remove(&key).unwrap();
            self.dumped_objects.insert(key, value.clone());
            self.current_pointer = key;

            self.write_align(value.alignment);
            self.write_label_declaration(value.label.clone());
            (value.dump)(value.start.load::<()>(), self); // TODO How to actually call this???
            self.advance_position(key + value.size);
            self.write_size(value.label);
            self.file.write_all("\n".as_bytes()).unwrap();
        }

        assert!(self.pending_references.is_empty()); // We should've dumped all referenced objects by now
        self.file.write_fmt(format_args!("\n#END DUMP OF {}\n", name)).unwrap();
    }
    #[inline]
    pub fn dump<T>(&mut self, name: String, value: &T) where T: Dump {
        self.dump_sized(name, value, mem::size_of::<T>(), mem::align_of::<T>())
    }
    #[inline]
    fn write_skip(&mut self, size: usize)  {
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n\t.skip {}", size)).unwrap();
    }

    #[inline]
    fn write_byte(&mut self, value: u8)  {
        match self.current_directive {
            // Continue the current .byte directive
            AsmDirective::Byte => self.file.write_fmt(format_args!{", {:#02x}", value}).unwrap(),
            _ => {
                self.current_directive = AsmDirective::Byte;
                // Start a new .byte directive
                self.file.write_fmt(format_args!{"\n\t.byte {:#02x}", value}).unwrap();
            }
        }
    }

    #[inline]
    fn write_align(&mut self, alignment: usize)  {
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n\t.balign {}", alignment)).unwrap();
    }

    #[inline]
    fn write_size(&mut self, label: AsmLabel)  {
        assert!(label.offset == 0);
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n\t.size {}, .-{}", label.base, label.base)).unwrap();
    }

    #[inline]
    fn write_equiv(&mut self, target: AsmLabel, source: AsmLabel)  {
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n\t.equiv {}, {}", target, source.offset(-target.offset))).unwrap();
    }

    #[inline]
    fn write_label_reference(&mut self, label: AsmLabel)  {
        match self.current_directive {
            // Continue the current ptr directive
            AsmDirective::Ptr => self.file.write_fmt(format_args!{", {}", label}).unwrap(),
            _ => {
                self.current_directive = AsmDirective::Ptr;
                // Start a new ptr directive
                if cfg!(target_arch = "x86_64") {
                    self.file.write_fmt(format_args!{"\n\t.quad {}", label}).unwrap();
                } else if cfg!(target_arch = "aarch64") {
                    self.file.write_fmt(format_args!{"\n\t.xword {}", label}).unwrap();
                } else {
                    unimplemented!(); // Other targets may have pointers of different sizes
                }
            }
        }
    }

    #[inline]
    fn write_global(&mut self, label: AsmLabel) {
        assert!(label.offset == 0);
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n\t.globl {}", label.base)).unwrap();
    }
    #[inline]
    fn write_label_declaration(&mut self, label: AsmLabel) {
        assert!(label.offset == 0);
        self.current_directive = AsmDirective::Other;
        self.file.write_fmt(format_args!("\n{}:", label.base)).unwrap();
    }

    #[inline]
    /// Advanced the current pointer to the specified address, adding padding as neccesary
    fn advance_position(&mut self, address: Address) {
        let padding = address - self.current_pointer;
        assert!(padding >= 0); //TODO: We can't go back... (OR CAN WE??)
        if padding != 0 {
            self.write_skip(padding as usize);
        }
        self.current_pointer += padding;
    }
}

// TODO: Implement tag...
// WARNING: Never dump an object of zero size (i.e. such an object should have a trivial dump method)
impl<W> rodal::Dumper for AsmDumper<W> where W: Write {
    /// Record the given complete object as needing to be dumped (because it is referenced)
    fn reference_object_sized<T>(&mut self, value: &T, size: usize, alignment: usize) where T: Dump {
        assert!(alignment != 0);
        let start = Address::new(value);

        // We already have a record for this object
        if self.dumped_objects.contains_key(&start) || self.pending_objects.contains_key(&start) {
            if cfg!(debug_assertions) {
                let object = if self.dumped_objects.contains_key(&start) {
                    self.dumped_objects.get(&start).unwrap()
                } else { // self.pending_objects.contains_key(&start)
                    self.pending_objects.get(&start).unwrap()
                };
                assert!(object.size == size && object.alignment == alignment);
            }
        } else {
            // This is the first time we've called reference_object on this pointer
            let label = AsmLabel::new_complete(start.value());

            // For each overlaping pending reference, update it's label and delete it
            // We can't iterate over a collection and delete simultaneusly
            // Also the insane borrow checker won't let me call write_equiv within the loop either
            let mut delete_keys: Vec<Address> = Vec::new(); // A list of keys to delete from pending_references
            let mut write_equiv_args: Vec<(AsmLabel, AsmLabel)> = Vec::new();
            for ptr in match self.pending_references.range_mut(start..start+size) {
                // Any reference that overlaps with a complete object should be entirely contained by that object
                write_equiv_args.push((AssmLabel::new_internal(ptr), label.offset(ptr - start)));
                delete_keys.push(ptr);
            }
            for (source, target) in write_equiv_args {
                self.write_equiv(source, target);
            }
            for key in delete_keys {
                self.pending_references.remove(&key);
            };

            // Value is suposed to be a new complete object, so verify it does
            // not overlap with any other complete objects
            debug_assert!(get_overlap(start, start+size, &mut self.dumped_objects).count() == 0);
            debug_assert!(get_overlap(start, start+size, &mut self.pending_objects).count() == 0);

            // TODO: For debuging purposes check this object is either disjoint from
            // or its memory area equals that of all other currently recorded objects
            self.pending_objects.insert(start, ObjectInfo::new(value, size, alignment, label));
        }
    }

    fn dump_reference_here<T>(&mut self, value: &&T, size: usize) {
        assert!(size != 0);
        let ptr = Address::new(*value);

        // Look for a recorded complete object containg this,..
        let label = match get_complete_object(ptr, ptr + size, &mut self.dumped_objects) {
            Some((_, value)) => value.label.offset(ptr - value.start),
            None => match get_complete_object(ptr, ptr + size, &mut self.pending_objects) {
                Some((_, value)) => value.label.offset(ptr - value.start),
                None => {
			// Just create a temporary label, and record our pointer
			self.pending_references.insert(ptr);
			AsmLabel::new_internal(ptr)
		}
            }
        };

        self.write_label_reference(label);
        // Pointerr & references should always have the same size, so theres no need to override this
        self.current_pointer += mem::size_of::<&&T>();
    }

    /// Dump the raw value of the object
    fn dump_value_sized_here<T>(&mut self, value: &T, size: usize) {
        let value = Address::new(value);
        let u8_size = mem::size_of::<u8>(); // Yes I know this equals 1
        for i in 0..size {
            // Write the byte
            self.write_byte(*(value + i*u8_size).load::<u8>());
            self.current_pointer += u8_size;
        }
    }

    /// Dump the given object (calls it's dump method)
    fn dump_object_here<T>(&mut self, value: &T) where T: Dump {
        value.dump::<Self>(self)
    }

    fn dump_padding_sized(&mut self, size: usize) {
        if size != 0 {
            self.write_skip(size);
        }
        self.current_pointer += size;
    }

    fn current_position(&self) -> usize {
        self.current_pointer.value()
    }
}

// Returns the range of elements in the map that overlaps with [start, end)
fn get_overlap<'a, W>(start: Address, end: Address, map: &'a mut BTreeMap<Address, ObjectInfo<W>>)
                       -> RangeMut<'a, Address, ObjectInfo<W>> where W: Write  {

    let start = match map.range_mut(..start).last() {
        Some((key, value)) =>
            if value.end() > start { *key }
                else { start },
        _ => start
    };
    map.range_mut(start..end)
}

// Gets the complete object that contains the range [start, end)
fn get_complete_object<'a, AR>(start: Address, end: Address, map: &'a mut BTreeMap<Address, AR>)
                               -> Option<(&'a Address, &mut AR)> where AR: AddressRange  {

    let mut overlap = get_overlap(start, end, map);
    let result = overlap.next();
    debug_assert!(overlap.count() == 0); // There should be at most one overlaping complete object

    //debug_assert!(result.as_ref().unwrap().1.start() <= start && end <= result.as_ref().unwrap().1.end());
    result
}

// Gets the largest address within the bounds of an element of map that overlaps with start
// (or returns None if nothing overlaps start)
fn get_overlap_end<AR>(start: Address, end: Address, map: &mut BTreeMap<Address, AR>)
                       -> Option<Address> where AR: AddressRange {

    // Find something that starts within [start, end)
    match map.range_mut(start..end).last() {
        Some((_, value)) => {
            return Some(value.end())
        }

        _ => {}
    }

    // Find the first thing that starts before [start, end)
    match map.range_mut(..start).last() {
        Some((_, value)) => {
            // Overlaps with [start, end)
            if value.end() > start { Some(value.end()) }
                else { None }
        },
        _ => None
    }
}

// Gets the first element (whcih is assumed to exist) that overlaps with [start, end)
fn get_first_overlap<'a, AR>(start: Address, end: Address, map: &'a mut BTreeMap<Address, AR>)
     -> &'a mut AR where AR: AddressRange {
    for (_, value) in map {
        if (value.start() <= start && value.end() >= start) ||
            (value.start() >= start && value.start() <= end) {
            return value;
        }
    }
    unreachable!();
}
