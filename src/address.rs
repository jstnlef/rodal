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

use std::fmt::*;
use std::ops::*;
use std::isize;
use std::mem;

// A utility trait that makes checks when casting
trait CheckedCast<T> {
    fn checked_cast(self) -> T;
}
impl CheckedCast<isize> for usize {
    fn checked_cast(self) -> isize {
        assert!(self <= isize::MAX as usize);
        self as isize
    }
}
impl CheckedCast<usize> for isize {
    fn checked_cast(self) -> usize {
        assert!(self >= 0);
        self as usize
    }
}

// Handles pointer operations for us
#[derive(Eq, Default, Hash, PartialOrd, Clone, Copy, PartialEq, Ord, Debug)]
pub struct Address(usize);
impl Address {
    pub fn null() -> Address { Address(0usize) }
    pub fn max() -> Address { Address(!0usize) }
    pub fn new<T: ?Sized>(value: &T) -> Address { Address(value as *const T as *const() as usize) }
    pub fn from_ptr<T: ?Sized>(value: *const T) -> Address { Address(value as *const() as usize) }
    pub fn to_ref<'a, T>(&self) -> &'a T { unsafe { mem::transmute(self.to_ptr::<T>())} }
    pub fn value(&self) -> usize { self.0 }
    pub fn to_ptr<T>(&self) -> *const T { self.0 as *const T }
}
impl Add<usize> for Address {
    type Output = Address;
    fn add(self, other: usize) -> Address { Address(self.0 + other) }
}
impl Add<isize> for Address {
    type Output = Address;
    fn add(self, other: isize) -> Address {
        if cfg!(debug_assertions) {
            // This is neccesary to make rust perform overflow checking approprietly
            if other >= 0 { Address(self.0 + other as usize) } else { Address(self.0 - other.wrapping_neg() as usize) }
        } else {
            Address(self.0.wrapping_add(other as usize))
        }
    }
}
impl AddAssign<usize> for Address {
    fn add_assign(&mut self, other: usize) { self.0 += other }
}
impl AddAssign<isize> for Address {
    fn add_assign(&mut self, other: isize) {
        if cfg!(debug_assertions) {
            if other >= 0 { self.0 += other as usize } else { self.0 -= other.wrapping_neg() as usize }
        } else {
            self.0 = self.0.wrapping_add(other as usize);
        }
    }
}
impl Sub<usize> for Address {
    type Output = Address;
    fn sub(self, other: usize) -> Address { Address(self.0 - other) }
}
impl Sub<isize> for Address {
    type Output = Address;
    fn sub(self, other: isize) -> Address {
        if cfg!(debug_assertions) {
            if other >= 0 { Address(self.0 - other as usize) } else { Address(self.0 + other.wrapping_neg() as usize) }
        } else {
            Address(self.0.wrapping_sub(other as usize))
        }
    }
}
impl SubAssign<usize> for Address {
    fn sub_assign(&mut self, other: usize) { self.0 -= other }
}
impl SubAssign<isize> for Address {
    fn sub_assign(&mut self, other: isize) {
        if other >= 0 { self.0 -= other as usize } else { self.0 += other.wrapping_neg() as usize }
    }
}
impl Sub<Address> for Address {
    type Output = isize;
    fn sub(self, other: Address) -> isize {
        if cfg!(debug_assertions) {
            if self.0 >= other.0 {
                (self.0 - other.0).checked_cast()
            } else {
                let res = other.0 - self.0; // Will be positive
                assert!(res <= isize::MIN.wrapping_neg() as usize);
                (res as isize).wrapping_neg()
            }
        } else {
            self.0.wrapping_sub(other.0) as isize
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:#018x}", self.0)
    }
}
/*impl Debug for Address {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "[{:16}]")
    }
}*/