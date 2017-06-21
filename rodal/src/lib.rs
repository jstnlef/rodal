#![feature(unique)]
#![feature(shared)]

pub unsafe trait Dump {
    /// Dump this object into the given RODAL Dumper
    /// WARNING: this function should only ever be called by a Dumper
    /// (use dump_object if you want to dump an object whilst dumping another one
    /// or use the Dumper's provided methods to start a dump)
    fn dump<D>(&self, dumper: &mut D) where D: Dumper;
}

use std::mem;
pub trait Dumper {
    /// Returns the address of the end of the last thing the dumper dumped
    fn current_position(&self) -> usize;

    fn dump_padding_sized(&mut self, size: usize);
    #[inline] fn dump_padding<T>(&mut self, target: &T) {
        let address = self.current_position();
        let target = (target as *const T) as usize;
        assert!(target >= address);
        self.dump_padding_sized(target - address);
    }

    fn dump_value_sized_here<T>(&mut self, value: &T, size: usize); // Core function
    #[inline] fn dump_value_sized<T>(&mut self, value: &T, size: usize) {
        self.dump_padding(value);
        self.dump_value_sized_here(value, size);
    }
    #[inline] fn dump_value_here<T>(&mut self, value: &T) {
        self.dump_value_sized_here(value, mem::size_of::<T>());
    }
    #[inline] fn dump_value<T>(&mut self, value: &T) {
        self.dump_padding(value);
        self.dump_value_sized_here(value, mem::size_of::<T>());
    }

    fn dump_object_here<T>(&mut self, value: &T) where T: Dump; // Core function
    #[inline] fn dump_object<T>(&mut self, value: &T) where T: Dump {
        //println!("\n#Dumping {}", value as *const T as usize);
        self.dump_padding(value);
        self.dump_object_here(value);
    }

    fn dump_reference_sized_here<T>(&mut self, value: &&T, size: usize); // Core Function
    #[inline] fn dump_reference_sized<T>(&mut self, value: &&T, size: usize) {
        self.dump_padding(value);
        self.dump_reference_sized_here(value, size);
    }
    #[inline] fn dump_reference_here<T>(&mut self, value: &&T) {
        self.dump_reference_sized_here(value, mem::size_of::<T>());
    }
    #[inline] fn dump_reference<T>(&mut self, value: &&T) {
        self.dump_padding(value);
        self.dump_reference_sized_here(value, mem::size_of::<T>());
    }

    fn reference_object_sized<T>(&mut self, value: &T, size: usize, alignment: usize) where T: Dump;
    #[inline] fn reference_object<T>(&mut self, value: &T) where T: Dump {
        self.reference_object_sized(value, mem::size_of::<T>(), mem::align_of::<T>())
    }

    #[inline] fn dump_reference_object_sized_here<T>(&mut self, value: &&T, size: usize, alignment: usize) where T: Dump {
        self.reference_object_sized(*value, size, alignment);
        self.dump_reference_sized_here(value, size);
    }
    #[inline] fn dump_reference_object_sized<T>(&mut self, value: &&T, size: usize, alignment: usize) where T: Dump {
        self.reference_object_sized(*value, size, alignment);
        self.dump_reference_sized(value, size);
    }
    #[inline] fn dump_reference_object_here<T>(&mut self, value: &&T) where T: Dump {
        self.reference_object(*value);
        self.dump_reference_here(value);
    }
    #[inline] fn dump_reference_object<T>(&mut self, value: &&T)  where T: Dump {
        self.reference_object(*value);
        self.dump_reference(value);
    }

}

pub mod std_impl;
