/*
Documentation:
(in order to use these you need a 'use rodal;' and 'use std;' statement)

Note: The folowing are used in the syntax given bellow for brevity
GEN := ['[' args* ']']
	optional generic args to be placed in the impl as <args...>
SOURCE: ['=' source]
	optional source type (defaulst to Self), the dump implementation will mem::transmute self to &source

rodal_value!(GEN ty):
	dumps ty as a raw sequence of bytes

rodal_object_referece!(GEN ty = &referant)
	dumps ty as if it refers to a complete object of type referant
rodal_pointer!(GEN ty = *referant)
	dumps ty as if it points to an instance of referant (which is not neccesarily a complete object)
rodal_object!(GEN ty = source)
    dump ty in the same way as source

rodal_struct!(GEN ty '{' $(field),* '}' SOURCE)
	dump self.fields... in the order specified.
	(fields are expressions, so you can use '0' for a tuple struct)
rodal_unordered_struct!(GEN ty '{' $(field),* '}' SOURCE)
	same as rodal_struct but will dump the fields in memory order not the order you specified
	(this is less efficient than rodal_struct! as it builds a BTreeMap)

rodal_enum!(GEN ty '{' $(variant),* '}' SOURCE)
	dumps an enum (will through unimplemented! for unspecified variants)
	variant can either be:
		unit
		    the name of a unit variant
		'(' tuple ':' $(element),+')'
		    element can be any distinct set of identifiers (the elements will be dumped in the order given)
		'{' struct ':' $(field),+'}'
		    field is the name of each field (they will be dumped in the order given)
rodal_unordered_enum!(GEN ty '{' $(variant),* '}' SOURCE)
	Same as rodal_enum! but will dump tuple and struct elements in memory order, not the order given

rodal_array_impl!(len)
	len should be a number, provides a generic implementation for all arrays
	of the form [T; len], you only need to use this if len > 32

rodal_tuple_impl!($(n: T),+)
	Each T should be a distinct identifier, and each n should be a number
	Ths will provided a generic implementation for tuples, dumping the elements
	in the order you specified 'n' in.
	You only need to use this for tuples with more than 16 elements

*/
use std;

#[macro_export]
macro_rules! rodal_value {
    ([$($gen:tt)*] $ty:ty) => [ rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        dumper.dump_value(fake_self);
    } = $ty} ];

    ($ty:ty) => [ rodal_value!{[] $ty} ];
}

#[macro_export]
macro_rules! rodal_object_reference {
    ([$($gen:tt)*] $ty:ty = &$referant:ty) => [ rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        let reference = unsafe{std::mem::transmute::<&Self, &&($referant)>(fake_self)};
        if std::mem::size_of_val(*reference) > 0 {
            dumper.dump_reference_object(reference);
        }
    } = $ty} ];

    ($ty:ty = &$referant:ty) => [ rodal_object_reference!{[] $ty = &$referant} ];
}

#[macro_export]
macro_rules! rodal_pointer {
    ([$($gen:tt)*] $ty:ty = *$referant:ty) => [rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        dumper.dump_reference(unsafe{std::mem::transmute::<&Self, &&($referant)>(fake_self)});
    } = $ty} ];

    ($ty:ty = *$referant:ty) => [ rodal_pointer!{[] $ty = *$referant} ];
}

#[macro_export]
macro_rules! rodal_object {
    ([$($gen:tt)*] $ty:ty = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        dumper.dump_object(fake_self);
    } = $source} ];

    ($ty:ty = $source:ty) => [ rodal_object!{[] $ty $source} ];
}


#[macro_export]
macro_rules! rodal_struct {
    ([$($gen:tt)*] $ty:ty {$($field:tt),*} = $source:ty) => [rodal___dump_impl!{(fake_self dumper D) [$($gen)*] $ty {
        $(dumper.dump_object(&fake_self.$field);)*;
    } = $source} ];

    ([$($gen:tt)*] $ty:ty {$($field:tt),*}) => [ rodal_struct!{[$($gen)*]$ty {$($field),*} = $ty} ];
    ($ty:ty {$($field:tt),*} = $source:ty) => [ rodal_struct!{[] $ty {$($field),*} = $source} ];
    ($ty:ty {$($field:tt),*}) => [ rodal_struct!{[] $ty {$($field),*} = $ty} ];
}

#[macro_export]
macro_rules! rodal_unordered_struct {
    ([$($gen:tt)*] $ty:ty {$($field:tt),*} = $source:ty) => [rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        let mut list = $crate::DumpList::<D>::new();
        $(list.add(&fake_self.$field);)*;
        list.dump(dumper);
    } = $source} ];

    ([$($gen:tt)*] $ty:ty {$($field:tt),*}) => [ rodal_unordered_struct!{[$($gen)*]$ty {$($field),*} = $ty} ];
    ($ty:ty {$($field:tt),*} = $source:ty) => [ rodal_unordered_struct!{[] $ty {$($field),*} = $source} ];
    ($ty:ty {$($field:tt),*}) => [ rodal_unordered_struct!{[] $ty {$($field),*} = $ty} ];
}

#[macro_export]
macro_rules! rodal_enum {
    ([$($gen:tt)*] $ty:ty {$($variant:tt),*} = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        match fake_self {
            $(rodal___variant_pattern!($variant) => {rodal___variant_impl!{(fake_self dumper D) $variant}})*
            _ => unimplemented!()
        }
    } = $source} ];

    ([$($gen:tt)*] $ty:ty {$($variant:tt),*}) => [ rodal_enum!{[$($gen)*]$ty {$($variant),*} = $ty} ];
    ($ty:ty {$($variant:tt),*} = $source:ty) => [ rodal_enum!{[] $ty {$($variant),*} = $source} ];
    ($ty:ty {$($variant:tt),*}) => [ rodal_enum!{[] $ty {$($variant),*} = $ty} ];
}
#[macro_export]
macro_rules! rodal_unordered_enum {
    ([$($gen:tt)*] $ty:ty {$($variant:tt),*} = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) [$($gen)*]$ty {
        match fake_self {
            $(rodal___variant_pattern!($variant) => {rodal___unordered_variant_impl!{(fake_self dumper D) $variant}})*
            _ => unimplemented!()
        }
    } = $source} ];

    ([$($gen:tt)*] $ty:ty {$($variant:tt),*}) => [ rodal_enum!{[$($gen)*]$ty {$($variant),*} = $ty} ];
    ($ty:ty {$($variant:tt),*} = $source:ty) => [ rodal_enum!{[] $ty {$($variant),*} = $source} ];
    ($ty:ty {$($variant:tt),*}) => [ rodal_enum!{[] $ty {$($variant),*} = $ty} ];
}

#[macro_export]
macro_rules! rodal___variant_pattern {
    ({$variant:ident : $field0:ident $(,$field:ident)*}) => [
        &$variant{ref $field0, $(ref $field,)*}
    ];
    (($variant:ident : $field0:ident $(,$field:ident)*)) => [
        &$variant(ref $field0, $(ref $field,)*)
    ];
    ($variant:ident) => [
        &$variant
    ];
}
#[macro_export]
macro_rules! rodal___variant_impl {
    (($fake_self:ident $dumper:ident $D:ident) {$variant:ident : $field0:ident $(,$field:ident)*}) => [
        $dumper.dump_prefix_value($field0);
        $dumper.dump_object($field0);
        $($dumper.dump_object($field);)*
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) ($variant:ident : $field0:ident $(,$field:ident)*)) => [
        $dumper.dump_prefix_value($field0);
        $dumper.dump_object($field0);
        $($dumper.dump_object($field);)*
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) $variant:ident) => [
        $dumper.dump_value($fake_self)
    ];
}

#[macro_export]
macro_rules! rodal___unordered_variant_impl {
    (($fake_self:ident $dumper:ident $D:ident) {$variant:ident : $field0:ident $(,$field:ident)*}) => [
        let mut list = $crate::DumpList::<$D>::new();
        $(list.add($field);)*
        $dumper.dump_prefix_value(list.first());
        list.dump($dumper);
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) ($variant:ident : $field0:ident $(,$field:ident)*)) => [
        let mut list = $crate::DumpList::<$D>::new();
        $(list.add($field);)*
        $dumper.dump_prefix_value(list.first());
        list.dump($dumper);
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) $variant:ident) => [
        $dumper.dump_value($fake_self)
    ];
}

#[macro_export]
macro_rules! rodal___dump_impl {
    (($fake_self:ident $dumper:ident $D:ident) [$($gen:tt)*] $ty:ty $body:block = $source:tt) => [
        #[allow(unreachable_patterns)]
        #[allow(unused_variables)]
        unsafe impl <$($gen)*> $crate::Dump for $ty {
            fn dump<$D: ?std::marker::Sized + $crate::Dumper>(&self, $dumper: &mut $D) {
                debug!("{}: {}::dump(dumper)", $crate::Address::new(self), stringify!($ty));
                let $fake_self: &($source) = unsafe{std::mem::transmute(self)};
                $body
            }
        }
    ];
}

#[macro_export]
macro_rules! rodal_array_impl {
    ($len:tt) => [ rodal___dump_impl!{(fake_self dumper D) [T: $crate::Dump] [T; $len] {
        for i in 0..$len {
            dumper.dump_object(&fake_self[i]);
        }
    } = [T; $len]} ];
}

#[macro_export]
macro_rules! rodal_tuple_impl {
    ($($n:tt : $ty:ident),*) => [ rodal___dump_impl!{(fake_self dumper D) [$($ty: $crate::Dump),*] ($($ty,)*) {
        $(dumper.dump_object(&fake_self.$n);)*
    } = ($($ty,)*)} ];
}

macro_rules! rodal___array_impls {
    ($($len:tt)+) => { $(rodal_array_impl!{$len})+ }
}
rodal___array_impls!(01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16
             17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32);

macro_rules! rodal___tuple_impls {
    ($(($( $n:tt: $ty:ident ),*))+) => { $(rodal_tuple_impl!{$($n : $ty),*})+ }
}
rodal___tuple_impls! {
    ()
    (0: T0)
    (0: T0, 1: T1)
    (0: T0, 1: T1, 2: T2)
    (0: T0, 1: T1, 2: T2, 3: T3)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11, 12: T12)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11, 12: T12, 13: T13)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11, 12: T12, 13: T13, 14: T14)
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11, 12: T12, 13: T13, 14: T14, 15: T15)
}
