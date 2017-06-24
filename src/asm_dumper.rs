use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::fmt;
use std::collections::btree_map::RangeMut;
use std::mem;
use std::collections::Bound;
use num::integer::lcm;
use super::*;
use std::iter::FromIterator;

#[derive(Clone, Default)]
struct AsmLabel {
    base: String,
    offset: isize,
}

impl AsmLabel {
    fn new(name: String) -> AsmLabel {
        AsmLabel {
            base: name,
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

struct ObjectInfo<W: Write> {
    start: Address,
    size: usize,
    alignment: usize,
    label: AsmLabel,
    value: Address, // The arg to pass to dump
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
    fn end(&self) -> Address { self.start + self.size}

    fn new<T: ?Sized, P: ?Sized>(value: &T, dump: DumpFunction<AsmDumper<W>>, position: &P, size: usize, alignment: usize, label: AsmLabel) -> ObjectInfo<W> {
        ObjectInfo {
            start: Address::new(position),
            size: size,
            label: label,
            alignment: alignment,
            value: Address::new(value),
            dump: dump,
        }
    }
}

#[cfg(target_arch = "x86_64")]
const POINTER_DIRECTIVE: &str = ".quad";
#[cfg(target_arch = "aarch64")]
const POINTER_DIRECTIVE: &str = ".xword";

enum AsmDirective {
    String,     // Whe are inside a .ascii
    Ptr,        // We are inside a POINTER_DIRECTIVE 
    Other       // we aran't inside either
}

pub struct AsmDumper<W: Write>
{
    file: W,
    current_pointer: Address, // This is the pointer into the output we are dumping
    current_directive: AsmDirective,

    /// Objects we've already dumped (or started to dump)
    dumped_objects: BTreeMap<Address, ObjectInfo<W>>,

    /// A table of all objects we haven't started dumping yet
    pending_objects: BTreeMap<Address, ObjectInfo<W>>,

    /// References that haven't been resolved to be relative to a complete object yet
    pending_references: BTreeSet<Address>,
    tags: BTreeMap<usize, Vec<*const ()>>,
}

impl<W: Write> AsmDumper<W> {
    pub fn new(mut file: W) -> AsmDumper<W> {
        file.write_all(b"#START RODAL DUMP\n\t.data\n").unwrap();
        AsmDumper::<W> {
            file: file,
            current_pointer: Address::null(),
            current_directive: AsmDirective::Other,
            dumped_objects: BTreeMap::new(),
            pending_objects: BTreeMap::new(),
            pending_references: BTreeSet::new(),
            tags: BTreeMap::new()
        }
    }
    pub fn dump_sized<T: ?Sized + Dump>(&mut self, name: &str, value: &T, size: usize, alignment: usize) -> &mut Self
    {
        assert!(alignment != 0);

        let start = Address::new(value);
        let label = AsmLabel::new(name.to_string());
        //trace!("{}: dump_sized({}, {}, {}, {})", self.current_pointer, name, start, size, alignment);

        self.current_pointer = Address::new(value);

        trace!("dumping {} [{}, {}):", label.base.clone(), start, start + size);
        self.write_global(label.clone());
        self.write_type_object(label.clone());
        self.write_size_align(size, alignment);
        self.write_label_declaration(label.clone());
        self.dumped_objects.insert(start, ObjectInfo::<W>::new(value, Self::get_dump_function::<T>(), value, size, alignment, label.clone()));
        value.dump::<Self>(self);
        self.advance_position(start + size); // Add any neccesary padding
        self.write_size(label);
        self
        // We finished dumping this root object
    }

    pub fn finish(&mut self) {
        //trace!("{:?}: finish()", self.current_pointer);

        // Still more objects to dump
        while !self.pending_objects.is_empty() {
            // Remove an element from the map (it dosn't mater which one)
            // And add a copy to dumped_objects
            let start = *self.pending_objects.keys().next().unwrap();
            let value = self.pending_objects.remove(&start).unwrap();
            self.dumped_objects.insert(start, value.clone());
            self.current_pointer = start;

            trace!("dumping {} [{}, {}):", value.label.base.clone(), start, start + value.size);
            self.write_type_object(value.label.clone());
            self.write_size_align(value.size, value.alignment);
            self.write_label_declaration(value.label.clone());
            self.dump_object_function_here(value.value.to_ref::<()>(), value.dump);
            self.advance_position(start + value.size);
            self.write_size(value.label);
        }

        // Dump the tags list (if there is one)
        let tags = self.tags.clone(); // Copy tags, so the borrow checker will let me mutate self
        if !tags.is_empty() {
            let vec = Vec::from_iter(tags.iter());
            self.dump("RODAL_TAGS", &vec);
        }

        assert!(self.pending_references.is_empty()); // We should've dumped all referenced objects by now

        // Write a label indicating the size and end of the rodal dump
        self.file.write_all(b"#END RODAL DUMP\n").unwrap();
    }

    #[inline]
    fn start_directive(&mut self, new_directive: AsmDirective) {
        match self.current_directive {
            AsmDirective::String => {self.file.write_all(b"\"\n").unwrap();}
            AsmDirective::Ptr => {self.file.write_all(b"\n").unwrap();}
            _ => {}
        }
        self.current_directive = new_directive;
    }
    #[inline]
    pub fn dump<T: ?Sized + Dump>(&mut self, name: &str, value: &T)  -> &mut Self {
        self.dump_sized(name, value, mem::size_of_val(value), mem::align_of_val(value))
    }
    #[inline]
    fn write_skip(&mut self, size: usize)  {
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.skip {}\n", size)).unwrap();
    }

    #[inline]
    fn write_byte(&mut self, value: u8)  {
        match self.current_directive {
            // Continue the current string directive
            AsmDirective::String => {},
            _ => {
                self.start_directive(AsmDirective::String);
                // Start a new string directive
                self.file.write_all(b"\t.ascii \"").unwrap();
            }
        }

        match value {
            // Some characters need to be escaped
            0x0a => self.file.write_all(b"\\\n").unwrap(),// Linefead
            0x5c => self.file.write_all(b"\\\\").unwrap(), // Backslash
            0x22 => self.file.write_all(b"\\\"").unwrap(),// Couble Quoutes
            _ => self.file.write_all(&[value]).unwrap(),
        }
    }

    #[inline]
    fn write_size(&mut self, label: AsmLabel)  {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.size {}, .-{}\n\n", label.base, label.base)).unwrap();
    }

    #[inline]
    fn write_equiv(&mut self, target: AsmLabel, source: AsmLabel)  {
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.equiv {}, {}\n", target.base, source.offset(-target.offset))).unwrap();
    }

    #[inline]
    fn write_label_reference(&mut self, label: AsmLabel)  {
        match self.current_directive {
            // Continue the current ptr directive
            AsmDirective::Ptr => self.file.write_fmt(format_args!{", {}", label}).unwrap(),
            _ => {
                self.start_directive(AsmDirective::Ptr);
                // Start a new ptr directive
                self.file.write_fmt(format_args!{"\t{} {}", POINTER_DIRECTIVE, label}).unwrap();
            }
        }
    }

    #[inline]
    // We need to write the size of objects so that we can handle it if
    // realloc is called on one
    fn write_size_align(&mut self, size: usize, alignment: usize)  {
        // We need to align to usize as we will store a usize indicating the size of the object
        let alignment = lcm(mem::align_of::<usize>(), alignment);
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.balign {}\n", alignment)).unwrap();

        // Add neccesary padding so that the data for the object is properly aligned
        let padding = alignment - mem::size_of::<usize>();
        if padding > 0 {
            self.file.write_fmt(format_args!("\t.skip {}\n", padding)).unwrap();
        }

        // Write the size, which will be aligned to mem::align_of::<usize>()
        self.file.write_fmt(format_args!{"\t{} {}\n", POINTER_DIRECTIVE, size}).unwrap();

        // Now the next thing that is written will be aligned to alignment
        // And have a properly aligned usize immediatly before it
    }

    #[inline]
    fn write_global(&mut self, label: AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.globl {}\n", label.base)).unwrap();
    }
    #[inline]
    fn write_type_object(&mut self, label: AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("\t.type {}, object\n", label.base)).unwrap();
    }
    #[inline]
    fn write_label_declaration(&mut self, label: AsmLabel) {
        assert!(label.offset == 0);
        self.start_directive(AsmDirective::Other);
        self.file.write_fmt(format_args!("{}:\n", label.base)).unwrap();
    }

    #[inline]
    /// Advanced the current pointer to the specified address, adding padding as neccesary
    fn advance_position(&mut self, address: Address) {
        //trace!("{:?}: advance_position({:?})", self.current_pointer, address);

        let padding = address - self.current_pointer;
        assert!(padding >= 0);
        if padding != 0 {
            self.write_skip(padding as usize);
        }
        self.current_pointer += padding;
        //trace!("+ {} -> {:?}", padding, self.current_pointer);
    }
}

// WARNING: Never dump an object of zero size (i.e. such an object should have a trivial dump method)
impl<W: Write> Dumper for AsmDumper<W> {
    fn tag_reference<T: ?Sized>(&mut self, value: &T, tag: usize) {
        //trace!("{:?}: tag_reference({:?}, {})", self.current_pointer, Address::new(value), tag);
        let value = value as *const T as *const();

        match self.tags.get_mut(&tag) {
            Some(tag_list) => return tag_list.push(value),
            None => { }
        }

        self.tags.insert(tag, vec![value]);
    }
    /// Record the given complete object as needing to be dumped (because it is referenced)
    fn reference_object_function_sized_position<T: ?Sized, P: ?Sized>(&mut self, value: &T, dump: DumpFunction<Self>, position: &P, size: usize, alignment: usize) {
        // Objects with zero size should never be referenced
        assert!(size != 0 && alignment != 0);
        let start = Address::new(position);
        //trace!("{:?}: reference_object_sized_position({}, {}, {}, {})", self.current_pointer, Address::new(value), start, size, alignment);

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
            let label = AsmLabel::new(format!("object_{}", start));

            // For each overlaping pending reference, update it's label and delete it
            // We can't iterate over a collection and delete simultaneusly
            // Also the insane borrow checker won't let me call write_equiv within the loop either
            let mut delete_keys: Vec<Address> = Vec::new(); // A list of keys to delete from pending_references
            let mut write_equiv_args: Vec<(AsmLabel, AsmLabel)> = Vec::new();
            for ptr in self.pending_references.range(start..start+size) {
                // Any reference that overlaps with a complete object should be entirely contained by that object
                write_equiv_args.push((AsmLabel::new(format!(".Lptr_{}", ptr)), label.offset(*ptr - start)));
                delete_keys.push(*ptr);
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

            self.pending_objects.insert(start, ObjectInfo::new(value, dump, position, size, alignment, label));
        }
    }

    fn dump_reference_here<T: ?Sized>(&mut self, value: &&T) {
        let ptr = Address::new(*value);
        //trace!("{:?}: dump_reference_here({:?} = &{})", self.current_pointer, Address::new(value), ptr);

        // Look for a recorded complete object containg this,..
        let label = match get_complete_object(ptr, &self.dumped_objects) {
            Some(value) => value.label.offset(ptr - value.start),
            None => match get_complete_object(ptr, &self.pending_objects) {
                Some(value) => value.label.offset(ptr - value.start),
                None => {
                    // Just create a temporary label, and record our pointer
                    self.pending_references.insert(ptr);
                    AsmLabel::new(format!(".Lptr_{}", ptr))
                }
            }
        };

        self.write_label_reference(label);
        // Pointerr & references should always have the same size, so theres no need to override this
        self.current_pointer += mem::size_of::<&&T>();
        //trace!("+ {} -> {:?}", mem::size_of::<&&T>(), self.current_pointer);
    }

    /// Dump the raw value of the object
    fn dump_value_sized_here<T: ?Sized>(&mut self, value: &T, size: usize) {
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

    fn current_position(&self) -> Address {
        self.current_pointer
    }

    fn dump_object_function_here(&mut self, value: &(), dump: DumpFunction<Self>) {
        let start = Address::new(value);
        //trace!("{:?}: dump_object_function_here({:?}, {})", self.current_pointer, start, unsafe{mem::transmute::<DumpFunction<Self>, Address>(dump)});
        (dump)(value, self);
    }
}

// Returns the range of elements in the map that overlaps with [start, end)
fn get_overlap<'a, W: Write>(start: Address, end: Address, map: &'a mut BTreeMap<Address, ObjectInfo<W>>)
                      -> RangeMut<'a, Address, ObjectInfo<W>> {

    let start = match map.range_mut(..start).last() {
        Some((key, value)) =>
            if value.end() > start { *key }
                else { start },
        _ => start
    };
    map.range_mut(start..end)
}

// Gets the complete object that contains start
fn get_complete_object<'a, W: Write>(start: Address, map: &'a BTreeMap<Address, ObjectInfo<W>>)
                              -> Option<&'a ObjectInfo<W>> {
    match map.range((Bound::Unbounded, Bound::Included(start))).last() {
        Some((_, value)) => {
            if value.end() > start {
                Some(value)
            } else {
                None
            }
        }
        None => None
    }
}