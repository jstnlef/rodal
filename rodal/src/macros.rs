#[macro_export]
macro_rules! rodal_value {
    ($ty:tt) => [ rodal_value!{[] $ty} ];
    ($gen:tt $ty:ty) => [ rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        dumper.dump_value(fake_self);
    } = $ty} ];
}

#[macro_export]
macro_rules! rodal_object_reference {
    ($ty:tt = &$referant:ty) => [ rodal_object_reference!{[] $ty = &$referant} ];
    ($gen:tt $ty:ty = &$referant:ty) => [ rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        dumper.dump_reference_object(unsafe{std::mem::transmute::<&Self, &&($referant)>(fake_self)});
    } = $ty} ];
}

#[macro_export]
macro_rules! rodal_pointer {
    ($ty:tt = *$referant:ty) => [ rodal_pointer!{[] $ty = *$referant} ];
    ($gen:tt $ty:ty = *$referant:ty) => [rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        dumper.dump_reference(unsafe{std::mem::transmute::<&Self, &&($referant)>(fake_self)});
    } = $ty} ];
}

#[macro_export]
macro_rules! rodal_object {
    ($ty:tt = $source:ty) => [ rodal_object!{[] $ty:tt $source} ];
    ($gen:tt $ty:ty = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        dumper.dump_object(fake_self);
    } = $source} ];
}


#[macro_export]
macro_rules! rodal_struct {
    ($ty:tt {$($body:tt)*} = $source:ty) => [ rodal___struct_impl!{[] $ty {$($body)*} = $source} ];
    ($ty:tt {$($body:tt)*}) => [ rodal___struct_impl!{[] $ty {$($body)*} = $ty} ];

    ($gen:tt $ty:ty {$($body:tt)*} = $source:ty) => [ rodal___struct_impl!{$gen $ty {$($body)*} = $source} ];
    ($gen:tt $ty:ty {$($body:tt)*}) => [ rodal___struct_impl!{$gen $ty {$($body)*} = $ty} ];

}

#[macro_export]
macro_rules! rodal___struct_impl {
    ($gen:tt $ty:tt ($($field:tt),*) = $source:tt) => [rodal___struct_impl!{$gen $ty {$($field),*} = $source} ];

    ($gen:tt $ty:tt {$($field:tt),*} = $source:tt) => [rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        $(dumper.dump_object(&fake_self.$field);)*;
    } = $source} ];
}

#[macro_export]
macro_rules! rodal_unordered_struct {
    ($ty:tt {$($body:tt)*} = $source:ty) => [ rodal___unordered_struct_impl!{[] $ty {$($body)*} = $source} ];
    ($ty:tt {$($body:tt)*}) => [ rodal___struct_impl!{[] $ty {$($body)*} = $ty} ];

    ($gen:tt $ty:ty {$($body:tt)*} = $source:ty) => [ rodal___unordered_struct_impl!{$gen $ty {$($body)*} = $source} ];
    ($gen:tt $ty:ty {$($body:tt)*}) => [ rodal___unordered_struct_impl!{$gen $ty {$($body)*} = $ty} ];

}

#[macro_export]
macro_rules! rodal___unordered_struct_impl {
    ($gen:tt $ty:tt ($($field:tt),*) = $source:tt) => [rodal___unordered_struct_impl!{$gen $ty {$($field),*} = $source} ];

    ($gen:tt $ty:tt {$($field:tt),*} = $source:tt) => [rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        let list = rodal::DumpList::<D>::new();
        $(list.add(&fake_self.$field);)*;
        list.dump(dumper);
    } = $source} ];
}


#[macro_export]
macro_rules! rodal_enum {
    ($gen:tt $ty:ty {$($variant:tt),*}) => [ rodal_enum!{$gen $ty {$($variant),*} = $ty} ];
    ($ty:tt {$($variant:tt),*} = $source:ty) => [ rodal_enum!{[] $ty {$($variant),*} = $source} ];
    ($ty:tt {$($variant:tt),*}) => [ rodal_enum!{[] $ty {$($variant),*} = $ty} ];
    ($gen:tt $ty:ty {$($variant:tt),*} = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        match fake_self {
            $(rodal___variant_pattern!($variant) => {rodal___variant_impl!{(fake_self dumper D) $variant}})*
            _ => unimplemented!()
        }
    } = $source} ];
}
macro_rules! rodal___variant_pattern {
    ({$variant:tt : $field0:tt $(,$field:tt)*}) => [
        &$variant{ref $field0, $(ref $field,)*}
    ];
    (($variant:tt : $field0:tt $(,$field:tt)*)) => [
        &$variant(ref $field0, $(ref $field,)*)
    ];
    ($variant:tt) => [
        &$variant
    ];
}
macro_rules! rodal___variant_impl {
    (($fake_self:ident $dumper:ident $D:ident) {$variant:tt : $field0:tt $(,$field:tt)*}) => [
        $dumper.dump_prefix_value($field0);
        $dumper.dump_object($field0);
        $($dumper.dump_object($field);)*
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) ($variant:tt : $field0:tt $(,$field:tt)*)) => [
        $dumper.dump_prefix_value($field0);
        $dumper.dump_object($field0);
        $($dumper.dump_object($field);)*
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) $variant:tt) => [
        $dumper.dump_value($fake_self)
    ];
}


#[macro_export]
macro_rules! rodal_unordered_enum {
    ($gen:tt $ty:ty {$($variant:tt),*}) => [ rodal_enum!{$gen $ty {$($variant),*} = $ty} ];
    ($ty:tt {$($variant:tt),*} = $source:ty) => [ rodal_enum!{[] $ty {$($variant),*} = $source} ];
    ($ty:tt {$($variant:tt),*}) => [ rodal_enum!{[] $ty {$($variant),*} = $ty} ];
    ($gen:tt $ty:ty {$($variant:tt),*} = $source:ty) => [ rodal___dump_impl!{(fake_self dumper D) $gen $ty {
        match fake_self {
            $(rodal___variant_pattern!($variant) => {rodal___unordered_variant_impl!{(fake_self dumper D) $variant}})*
            _ => unimplemented!()
        }
    } = $source} ];
}

#[macro_export]
macro_rules! rodal___unordered_variant_impl {
    (($fake_self:ident $dumper:ident $D:ident) {$variant:tt : $field0:tt $(,$field:tt)*}) => [
        let list = rodal::DumpList::<$D>::new();
        $(list.add($field);)*
        $dumper.dump_prefix_value(list.first());
        list.dump($dumper)
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) ($variant:tt : $field0:tt $(,$field:tt)*)) => [
        let list = rodal::DumpList::<$D>::new();
        $(list.add($field);)*
        $dumper.dump_prefix_value(list.first());
        list.dump($dumper)
        $dumper.dump_suffix_value($fake_self);
    ];
    (($fake_self:ident $dumper:ident $D:ident) $variant:tt) => [
        $dumper.dump_value($fake_self)
    ];
}

#[macro_export]
macro_rules! rodal___dump_impl {
    (($fake_self:ident $dumper:ident $D:ident) [$($gen:tt)*] $ty:tt  $body:block = $source:tt) => [
        #[allow(unreachable_patterns)]
        #[allow(unused_variables)]
        unsafe impl <$($gen)*> rodal::Dump for $ty {
            fn dump<$D: ?std::marker::Sized + rodal::Dumper>(&self, $dumper: &mut $D) {
                trace!("{}: {}::dump(dumper)", rodal::Address::new(self), stringify!($ty));
                let $fake_self: &($source) = unsafe{std::mem::transmute(self)};
                $body
            }
        }
    ];
}

#[macro_export]
macro_rules! rodal_array_impl {
    ($len:tt) => [ rodal___dump_impl!{(fake_self dumper D) [T: Dump] [T; $len] {
        for i in 0..$len {
            dumper.dump_object(&fake_self[i]);
        }
    } = [T; $len]} ];
}

#[macro_export]
macro_rules! rodal_tuple_impl {
    ($($n:tt $name:ident)+) => [ rodal___dump_impl!{(fake_self dumper D) [$($name: Dump),+] ($($name,)+) {
        $(dumper.dump_object(&fake_self.$n);)+
    } = ($($name,)+)} ];
}