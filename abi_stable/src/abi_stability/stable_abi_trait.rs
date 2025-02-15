/*!
Where the StableAbi trait is declared,as well as related types/traits.
*/

use core_extensions::type_level_bool::{Boolean, False, True};
use std::{
    cell::{Cell,UnsafeCell},
    cmp::{Eq,PartialEq},
    fmt,
    marker::{PhantomData,PhantomPinned},
    mem::ManuallyDrop,
    num::{NonZeroU8,NonZeroU16,NonZeroU32,NonZeroU64,NonZeroUsize,Wrapping},
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, AtomicUsize},
};

use crate::{
    abi_stability::get_static_equivalent::GetStaticEquivalent_,
    std_types::{RNone, RSome, utypeid::UTypeId},
    sabi_types::ReturnValueEquality,
    reflection::ModReflMode,
    type_layout::{
        LifetimeIndex, TLData, TLField, TypeLayout, TypeLayoutParams,
        ItemInfo,ReprAttr,TLPrimitive,TLEnum,TLDiscriminants,IsExhaustive,
    },
};


///////////////////////

/**
Represents a type whose layout is stable.

This trait can indirectly be derived using `#[derive(StableAbi)]`
(There is a blanket impl of StableAbi for `SharedStableAbi<Kind=ValueKind>`,
    which `#[derive(StableAbi)]` implements.
).

There is a blanket impl of this trait for all `SharedStableAbi<Kind=ValueKind>` types.
*/
pub unsafe trait StableAbi:SharedStableAbi<Kind=ValueKind> {
    /// The layout of the type provided by implementors.
    const LAYOUT: &'static TypeLayout;

    /// The layout of the type,derived from Self::LAYOUT and associated types.
    const ABI_INFO: &'static AbiInfoWrapper;
}


/**
Represents a type whose layout is stable.

This trait can be derived using ``.

# Safety

The layout of types implementing this trait can only change by 
adding fields at the end,if it stores a `TLData::PrefixType` in `TypeLayout.data`,

# Caveats

This trait cannot be directly implemented for functions that take lifetime parameters,
because of that,`#[derive(StableAbi)]` detects the presence of `extern fn` types 
in type definitions.

*/
pub unsafe trait SharedStableAbi:GetStaticEquivalent_ {
    /**
Whether this type has a single invalid bit-pattern.

Possible values:True/False

Some standard library types have a single value that is invalid for them eg:0,null.
these types are the only ones which can be stored in a `Option<_>` that implements AbiStable.

An alternative for types where `IsNonZeroType=False`,you can use `abi_stable::ROption`.

Non-exhaustive list of std types that are NonZero:

- &T (any T).

- &mut T (any T).

- extern fn().

- std::ptr::NonNull

- std::num::NonZero* 

    */
    type IsNonZeroType: Boolean;

    /**
The kind of abi stability of this type,there are 2:

- ValueKind:The layout of this type does not change in minor versions.

- PrefixKind:
    A struct which can add fields in minor versions,
    only usable behind a shared reference,
    used to implement extensible vtables and modules.

    */
    type Kind:TypeKindTrait;

    /// The layout of the type provided by implementors.
    const S_LAYOUT: &'static TypeLayout;

    /// The layout of the type,derived from Self::LAYOUT and associated types.
    const S_ABI_INFO: &'static AbiInfoWrapper = {
        let info = AbiInfo {
            kind:<Self::Kind as TypeKindTrait>::VALUE,
            prefix_kind: <Self::Kind as TypeKindTrait>::IS_PREFIX,
            type_id:make_rve_utypeid!(Self::StaticEquivalent),
            is_nonzero: <Self::IsNonZeroType as Boolean>::VALUE,
            layout: Self::S_LAYOUT,
        };

        &AbiInfoWrapper::new(info)
    };
}


unsafe impl<This> StableAbi for This
where 
    This:SharedStableAbi<Kind=ValueKind>,
{
    const LAYOUT: &'static TypeLayout=<This as SharedStableAbi>::S_LAYOUT;
    const ABI_INFO: &'static AbiInfoWrapper=<This as SharedStableAbi>::S_ABI_INFO;
}


///////////////////////

/// Wraps a correctly constructed AbiInfo.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[derive(StableAbi)]
pub struct AbiInfoWrapper {
    inner: AbiInfo,
    _priv: (),
}

impl AbiInfoWrapper {
    const fn new(inner: AbiInfo) -> Self {
        Self { inner, _priv: () }
    }
    /// Unsafely constructs AbiInfoWrapper from any AbiInfo.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the layout is that of the datatype this AbiInfo represents,
    /// and that it stays consistent with it across time.
    pub const unsafe fn new_unchecked(inner: AbiInfo) -> Self {
        Self::new(inner)
    }
    /// Gets the wrapped AbiInfo.
    pub const fn get(&self) -> &AbiInfo {
        &self.inner
    }
}

/// Describes the abi of some type.
///
/// # Safety
///
/// You must ensure that it describes the actual abi of the type.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
#[derive(StableAbi)]
pub struct AbiInfo {
    pub kind:TypeKind,
    /// Whether this is a prefix,
    pub prefix_kind: bool,
    /// Equivalent to the UTypeId returned by the function in ReturnValueEquality.
    pub type_id:ReturnValueEquality<UTypeId>,
    /// Whether the type uses non-zero value optimization,
    /// if true then an Option<Self> implements StableAbi.
    pub is_nonzero: bool,
    /// The layout of the type.
    pub layout: &'static TypeLayout,
}


impl AbiInfo{
    /// Gets the UTypeId for the `'static` equivalent of the type that created this AbiInfo.
    pub fn get_utypeid(&self)->UTypeId{
        (self.type_id.function)()
    }
}

///////////////////////////////////////////////////////////////////////////////

/// The abi_stable kind of a type.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq,Eq,Ord,PartialOrd,Hash,StableAbi)]
pub enum TypeKind{
    /// A value whose layout must not change in minor versions
    Value,
    /// A struct whose fields can be extended in minor versions,
    /// but only behind a shared reference,
    /// used to implement vtables and modules.
    Prefix,
}


/// For marker types that represent variants of TypeKind.
pub trait TypeKindTrait:sealed::Sealed{
    /// The equivalent TypeKind of this type.
    const VALUE:TypeKind;
    /// Whether this is a prefix-kind
    const IS_PREFIX:bool;
}

/// The kind of a regular value,the default kind.
pub struct ValueKind;

/// The kind of a prefix-type,vtables and modules.
pub struct PrefixKind;


mod sealed{
    pub trait Sealed{}
}

impl sealed::Sealed for ValueKind{}
impl sealed::Sealed for PrefixKind{}

impl TypeKindTrait for ValueKind {
    const VALUE:TypeKind=TypeKind::Value;
    const IS_PREFIX:bool=false;
}

impl TypeKindTrait for PrefixKind {
    const VALUE:TypeKind=TypeKind::Prefix;
    const IS_PREFIX:bool=true;
}

///////////////////////////////////////////////////////////////////////////////

/// Gets for the AbiInfo of some type,wraps an `extern "C" fn() -> &'static AbiInfo`.
#[derive(Copy, Clone)]
#[repr(transparent)]
#[derive(StableAbi)]
// #[sabi(debug_print)]
pub struct GetAbiInfo {
    abi_info: extern "C" fn() -> &'static AbiInfo,
}

impl fmt::Debug for GetAbiInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl GetAbiInfo {
    /// Gets the `&'static AbiInfo` of some type.
    pub fn get(self) -> &'static AbiInfo {
        (self.abi_info)()
    }
}

impl Eq for GetAbiInfo{}

impl PartialEq for GetAbiInfo{
    fn eq(&self,other:&Self)->bool{
        self.get()==other.get()
    }
}


/// Constructs the GetAbiInfo for Self.
///
/// # Safety
///
/// Implementors must make sure that the AbiInfo actually describes the layout of the type.
pub unsafe trait MakeGetAbiInfo<B> {
    const CONST: GetAbiInfo;
}

unsafe impl<T> MakeGetAbiInfo<StableAbi_Bound> for T
where
    T: StableAbi,
{
    const CONST: GetAbiInfo = GetAbiInfo {
        abi_info: get_abi_info::<T>,
    };
}

unsafe impl<T> MakeGetAbiInfo<SharedStableAbi_Bound> for T
where
    T: SharedStableAbi,
{
    const CONST: GetAbiInfo = GetAbiInfo {
        abi_info: get_ssa_abi_info::<T>,
    };
}

unsafe impl<T> MakeGetAbiInfo<UnsafeOpaqueField_Bound> for T {
    const CONST: GetAbiInfo = GetAbiInfo {
        abi_info: get_abi_info::<UnsafeOpaqueField<T>>,
    };
}


#[doc(hidden)]
pub struct MakeGetAbiInfoSA<T>(T);

impl<T> MakeGetAbiInfoSA<T>
where T: StableAbi,
{
    pub const STABLE_ABI:GetAbiInfo=GetAbiInfo {
        abi_info: get_abi_info::<T>,
    };
}


#[doc(hidden)]
pub struct MakeGetAbiInfoUF<T>(T);

impl<T> MakeGetAbiInfoUF<T>{
    pub const OPAQUE_FIELD:GetAbiInfo=GetAbiInfo {
        abi_info: get_abi_info::<UnsafeOpaqueField<T>>,
    };
}


/// Determines that MakeGetAbiInfo constructs the AbiInfo for a 
/// type that implements StableAbi.
#[allow(non_camel_case_types)]
pub struct StableAbi_Bound;

/// Determines that MakeGetAbiInfo constructs the AbiInfo for a 
/// type that implements SharedStableAbi.
#[allow(non_camel_case_types)]
pub struct SharedStableAbi_Bound;

/// Determines that MakeGetAbiInfo constructs the AbiInfo for any type (this is unsafe).
#[allow(non_camel_case_types)]
pub struct UnsafeOpaqueField_Bound;

/// Retrieves the AbiInfo of `T:StableAbi`,
pub extern "C" fn get_abi_info<T>() -> &'static AbiInfo
where
    T: StableAbi,
{
    T::ABI_INFO.get()
}

/// Retrieves the AbiInfo of `T:SharedStableAbi`,
pub extern "C" fn get_ssa_abi_info<T>() -> &'static AbiInfo
where
    T: SharedStableAbi,
{
    T::S_ABI_INFO.get()
}


///////////////////////////////////////////////////////////////////////////////

/////////////////////////////////////////////////////////////////////////////
////                Implementations
/////////////////////////////////////////////////////////////////////////////

///////////////////////////////////////////////////////////////////////////////

unsafe impl<T> GetStaticEquivalent_ for PhantomData<T> 
where T:GetStaticEquivalent_
{
    type StaticEquivalent=PhantomData<T::StaticEquivalent>;
}

unsafe impl<T> SharedStableAbi for PhantomData<T> 
where T:StableAbi
{
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "PhantomData",
        RNone,
        ItemInfo::std_type_in("std::marker"),
        TLData::EMPTY,
        ReprAttr::c(),
        tl_genparams!(;;),
        &[TLField::new("0",&[],<T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,)],
    );
}


unsafe impl GetStaticEquivalent_ for () {
    type StaticEquivalent=();
}
unsafe impl SharedStableAbi for () {
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout =
        &TypeLayout::from_std::<Self>(
            "()", 
            TLData::EMPTY,
            ReprAttr::c(),
            ItemInfo::primitive(), 
            tl_genparams!(;;)
        );
}


/////////////


unsafe impl<'a, T> GetStaticEquivalent_ for &'a T
where
    T: 'a + GetStaticEquivalent_,
{
    type StaticEquivalent=&'static T::StaticEquivalent;
}

// Does not allow ?Sized types because the DST fat pointer does not have a stable layout.
unsafe impl<'a, T> SharedStableAbi for &'a T
where
    T: 'a + SharedStableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = True;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "&",
        RSome(TLPrimitive::SharedRef),
        ItemInfo::primitive(),
        TLData::Primitive(TLPrimitive::SharedRef),
        ReprAttr::Primitive,
        tl_genparams!('a;T;),
        &[TLField::new(
            "0",
            &[LifetimeIndex::Param(0)],
            <T as MakeGetAbiInfo<SharedStableAbi_Bound>>::CONST,
        )],
    ).set_mod_refl_mode(ModReflMode::DelegateDeref{phantom_field_index:0});
}


unsafe impl<'a, T> GetStaticEquivalent_ for &'a mut T
where
    T: 'a + GetStaticEquivalent_,
{
    type StaticEquivalent=&'static mut T::StaticEquivalent;
}

// Does not allow ?Sized types because the DST fat pointer does not have a stable layout.
unsafe impl<'a, T> SharedStableAbi for &'a mut T
where
    T: 'a + StableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = True;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "&mut",
        RSome(TLPrimitive::MutRef),
        ItemInfo::primitive(),
        TLData::Primitive(TLPrimitive::MutRef),
        ReprAttr::Primitive,
        tl_genparams!('a;T;),
        &[TLField::new(
            "0",
            &[LifetimeIndex::Param(0)],
            <T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
        )],
    );
}


unsafe impl<T> GetStaticEquivalent_ for NonNull<T>
where
    T: GetStaticEquivalent_,
{
    type StaticEquivalent=NonNull<T::StaticEquivalent>;
}

// Does not allow ?Sized types because the DST fat pointer does not have a stable layout.
unsafe impl<T> SharedStableAbi for NonNull<T>
where
    T: StableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = True;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "NonNull",
        RNone,
        ItemInfo::std_type_in("std::ptr"),
        TLData::struct_(&[
            TLField::new(
                "0",
                &[],
                <*mut T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
            )
        ]),
        ReprAttr::Transparent,
        tl_genparams!(;T;),
        &[],
    );
}


unsafe impl<T> GetStaticEquivalent_ for AtomicPtr<T>
where
    T: GetStaticEquivalent_,
{
    type StaticEquivalent=AtomicPtr<T::StaticEquivalent>;
}

unsafe impl<T> SharedStableAbi for AtomicPtr<T>
where
    T: StableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "AtomicPtr",
        RNone,
        ItemInfo::std_type_in("std::sync::atomic"),
        TLData::struct_(&[
            TLField::new(
                "0",
                &[],
                <*mut T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
            )
        ]),
        ReprAttr::Transparent,
        tl_genparams!(;T;),
        &[],
    );
}

unsafe impl<T> GetStaticEquivalent_ for *const T
where
    T: GetStaticEquivalent_,
{
    type StaticEquivalent=*const T::StaticEquivalent;
}
// Does not allow ?Sized types because the DST fat pointer does not have a stable layout.
unsafe impl<T> SharedStableAbi for *const T
where
    T: SharedStableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "*const",
        RSome(TLPrimitive::ConstPtr),
        ItemInfo::primitive(),
        TLData::Primitive(TLPrimitive::ConstPtr),
        ReprAttr::Primitive,
        tl_genparams!(;T;),
        &[TLField::new(
            "0",
            &[],
            <T as MakeGetAbiInfo<SharedStableAbi_Bound>>::CONST,
        )],
    );
}


unsafe impl<T> GetStaticEquivalent_ for *mut T
where
    T: GetStaticEquivalent_,
{
    type StaticEquivalent=*mut T::StaticEquivalent;
}
// Does not allow ?Sized types because the DST fat pointer does not have a stable layout.
unsafe impl<T> SharedStableAbi for *mut T
where
    T: StableAbi,
{
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "*mut",
        RSome(TLPrimitive::MutPtr),
        ItemInfo::primitive(),
        TLData::Primitive(TLPrimitive::MutPtr),
        ReprAttr::Primitive,
        tl_genparams!(;T;),
        &[TLField::new(
            "0",
            &[],
            <T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
        )],
    );
}

/////////////

macro_rules! impl_stable_abi_array {
    ($($size:expr),*)=>{
        $(
            unsafe impl<T> GetStaticEquivalent_ for [T;$size]
            where T:GetStaticEquivalent_
            {
                type StaticEquivalent=[T::StaticEquivalent;$size];
            }

            unsafe impl<T> SharedStableAbi for [T;$size]
            where T:StableAbi
            {
                type Kind=ValueKind;
                type IsNonZeroType=False;

                const S_LAYOUT:&'static TypeLayout=&TypeLayout::from_std_full::<Self>(
                    "array",
                    RSome(TLPrimitive::Array{len:$size}),
                    ItemInfo::primitive(),
                    TLData::Primitive(TLPrimitive::Array{len:$size}),
                    ReprAttr::Primitive,
                    tl_genparams!(;T;$size),
                    &[
                        TLField::new(
                            "element", 
                            &[],
                            <T as MakeGetAbiInfo<SharedStableAbi_Bound>>::CONST
                        )
                    ],
                );
            }
        )*
    }
}

impl_stable_abi_array! {
    00,01,02,03,04,05,06,07,08,09,
    10,11,12,13,14,15,16,17,18,19,
    20,21,22,23,24,25,26,27,28,29,
    30,31,32
}

/////////////

unsafe impl<T> GetStaticEquivalent_ for Option<T>
where
    T: GetStaticEquivalent_,
{
    type StaticEquivalent=Option<T::StaticEquivalent>;
}
/// Implementing abi stability for Option<T> is fine if
/// T is a NonZero primitive type.
unsafe impl<T> SharedStableAbi for Option<T>
where
    T: StableAbi<IsNonZeroType = True>,
{
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std_full::<Self>(
        "Option",
        RNone,
        ItemInfo::primitive(),
        TLData::Enum(&TLEnum::new(
            "Some;None;",
            IsExhaustive::exhaustive(),
            &[
                TLField::new(
                    "0",
                    &[],
                    <T as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
                )
            ],
            TLDiscriminants::from_u8_slice(&[0,1]),
            &[1,0]
        )),
        ReprAttr::OptionNonZero,
        tl_genparams!(;T;),
        &[],
    );
}

/////////////


macro_rules! impl_for_primitive_ints {
    (
        $( ($zeroable:ty,$tl_primitive:expr) ,)*
    ) => (
        $(
            unsafe impl GetStaticEquivalent_ for $zeroable {
                type StaticEquivalent=Self;
            }
            unsafe impl SharedStableAbi for $zeroable {
                type Kind=ValueKind;
                type IsNonZeroType=False;

                const S_LAYOUT:&'static TypeLayout=&TypeLayout::from_std_full::<Self>(
                    stringify!($zeroable),
                    RSome($tl_primitive),
                    ItemInfo::primitive(),
                    TLData::Primitive($tl_primitive),
                    ReprAttr::Primitive,
                    tl_genparams!(;;),
                    &[],
                );
            }
        )*
    )
}

impl_for_primitive_ints!{
    (u8   ,TLPrimitive::U8),
    (i8   ,TLPrimitive::I8),
    (u16  ,TLPrimitive::U16),
    (i16  ,TLPrimitive::I16),
    (u32  ,TLPrimitive::U32),
    (i32  ,TLPrimitive::I32),
    (u64  ,TLPrimitive::U64),
    (i64  ,TLPrimitive::I64),
    (usize,TLPrimitive::Usize),
    (isize,TLPrimitive::Isize),
    (bool ,TLPrimitive::Bool),
}


macro_rules! impl_for_concrete {
    (
        type IsNonZeroType=$zeroness:ty;
        [
            $( ($this:ty,$prim_repr:ty,$in_mod:expr) ,)*
        ]
    ) => (
        $(
            unsafe impl GetStaticEquivalent_ for $this {
                type StaticEquivalent=Self;
            }
            unsafe impl SharedStableAbi for $this {
                type Kind=ValueKind;
                type IsNonZeroType=$zeroness;

                const S_LAYOUT:&'static TypeLayout=&TypeLayout::from_std::<Self>(
                    stringify!($this),
                    TLData::struct_(&[
                        TLField::new(
                            "0",
                            &[],
                            <$prim_repr as MakeGetAbiInfo<StableAbi_Bound>>::CONST,
                        )
                    ]),
                    ReprAttr::Transparent,
                    ItemInfo::std_type_in($in_mod),
                    tl_genparams!(;;),
                );
            }
        )*
    )
}



impl_for_concrete! {
    type IsNonZeroType=False;
    [
        (AtomicBool ,bool,"std::sync::atomic"),
        (AtomicIsize,isize,"std::sync::atomic"),
        (AtomicUsize,usize,"std::sync::atomic"),
    ]
}

impl_for_concrete! {
    type IsNonZeroType=True;
    [
        (NonZeroU8   ,u8,"std::num"),
        (NonZeroU16  ,u16,"std::num"),
        (NonZeroU32  ,u32,"std::num"),
        (NonZeroU64  ,u64,"std::num"),
        (NonZeroUsize,usize,"std::num"),
    ]
}
/////////////


#[cfg(any(rust_1_34,feature="rust_1_34"))]
mod rust_1_34_impls{
    use super::*;
    use std::sync::atomic::*;
    use core::num::*;

    impl_for_concrete! {
        type IsNonZeroType=False;
        [
            (AtomicI8 ,i8,"std::sync::atomic"),
            (AtomicI16,i16,"std::sync::atomic"),
            (AtomicI32,i32,"std::sync::atomic"),
            (AtomicI64,i64,"std::sync::atomic"),
            (AtomicU8 ,u8,"std::sync::atomic"),
            (AtomicU16,u16,"std::sync::atomic"),
            (AtomicU32,u32,"std::sync::atomic"),
            (AtomicU64,u64,"std::sync::atomic"),
        ]
    }

    impl_for_concrete! {
        type IsNonZeroType=True;
        [
            (NonZeroI8   ,i8,"core::num"),
            (NonZeroI16  ,i16,"core::num"),
            (NonZeroI32  ,i32,"core::num"),
            (NonZeroI64  ,i64,"core::num"),
            (NonZeroIsize,isize,"core::num"),
        ]
    }
}

/////////////

macro_rules! impl_stableabi_for_repr_transparent {
    (
        $type_constr:ident
        $(where[ $($where_clause:tt)* ])* ,
        $item_info:expr
    ) => (
        unsafe impl<P> GetStaticEquivalent_ for $type_constr<P>
        where
            P: GetStaticEquivalent_,
            $($($where_clause)*)*
        {
            type StaticEquivalent=$type_constr<P::StaticEquivalent>;
        }
        unsafe impl<P> SharedStableAbi for $type_constr<P>
        where
            P: StableAbi,
            $($($where_clause)*)*
        {
            type Kind=ValueKind;
            type IsNonZeroType = P::IsNonZeroType;

            const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std::<Self>(
                stringify!($type_constr),
                TLData::struct_(&[
                    TLField::new("0",&[],<P as MakeGetAbiInfo<StableAbi_Bound>>::CONST,)
                ]),
                ReprAttr::Transparent,
                $item_info,
                tl_genparams!(;P;),
            );
        }
    )
}


impl_stableabi_for_repr_transparent!{ Wrapping ,ItemInfo::std_type_in("std::num") }
impl_stableabi_for_repr_transparent!{ Pin         ,ItemInfo::std_type_in("std::pin") }
impl_stableabi_for_repr_transparent!{ ManuallyDrop,ItemInfo::std_type_in("std::mem") }
impl_stableabi_for_repr_transparent!{ Cell        ,ItemInfo::std_type_in("std::cell") }
impl_stableabi_for_repr_transparent!{ UnsafeCell  ,ItemInfo::std_type_in("std::cell") }

/////////////

macro_rules! impl_stableabi_for_unit_struct {
    (
        $type_constr:ident,
        $item_info:expr
    ) => (
        unsafe impl GetStaticEquivalent_ for $type_constr{
            type StaticEquivalent=$type_constr;
        }
        unsafe impl SharedStableAbi for $type_constr{
            type Kind=ValueKind;
            type IsNonZeroType = False;

            const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_std::<Self>(
                stringify!($type_constr),
                TLData::EMPTY,
                ReprAttr::c(),
                $item_info,
                tl_genparams!(;;),
            );
        }
    )
}


impl_stableabi_for_unit_struct!{ PhantomPinned,ItemInfo::std_type_in("std::marker") }

/////////////


unsafe impl GetStaticEquivalent_ for core_extensions::Void {
    type StaticEquivalent=Self;
}
unsafe impl SharedStableAbi for core_extensions::Void {
    type Kind=ValueKind;
    type IsNonZeroType = False;


    const S_LAYOUT: &'static TypeLayout =
        &TypeLayout::from_params::<Self>(TypeLayoutParams {
            name: "Void",
            item_info:ItemInfo::package_and_mod("core_extensions;0.0.0","core_extensions"),
            data: TLData::Enum(&TLEnum::new(
                "",
                IsExhaustive::exhaustive(),
                &[],
                TLDiscriminants::from_u8_slice(&[]),
                &[]
            )),
            generics: tl_genparams!(;;),
        });
}



/////////////

/// The layout of `extern fn()` and `unsafe extern fn()`
macro_rules! empty_extern_fn_layout{
    ($this:ty) => (
        &TypeLayout::from_params::<extern "C" fn()>(TypeLayoutParams {
            name: "AFunctionPointer",
            item_info:make_item_info!(),
            data: TLData::struct_(&[]),
            generics: tl_genparams!(;;),
        })
    )
}


/// This is the only function type that implements StableAbi
/// so as to make it more obvious that functions involving lifetimes
/// cannot implement this trait directly (because of higher ranked trait bounds).
unsafe impl GetStaticEquivalent_ for extern "C" fn() {
    type StaticEquivalent=Self;
}
unsafe impl SharedStableAbi for extern "C" fn() {
    type Kind=ValueKind;
    type IsNonZeroType = True;

    const S_LAYOUT: &'static TypeLayout = empty_extern_fn_layout!(Self);
}

/// This is the only function type that implements StableAbi
/// so as to make it more obvious that functions involving lifetimes
/// cannot implement this trait directly (because of higher ranked trait bounds).
unsafe impl GetStaticEquivalent_ for unsafe extern "C" fn() {
    type StaticEquivalent=Self;
}
unsafe impl SharedStableAbi for unsafe extern "C" fn() {
    type Kind=ValueKind;
    type IsNonZeroType = True;

    const S_LAYOUT: &'static TypeLayout = empty_extern_fn_layout!(Self);
}


/// The GetAbiInfo of an `unsafe extern fn()`
pub const UNSAFE_EXTERN_FN_ABI_INFO:GetAbiInfo=MakeGetAbiInfoSA::<unsafe extern fn()>::STABLE_ABI;

/// The GetAbiInfo of an `extern fn()`
pub const EXTERN_FN_ABI_INFO:GetAbiInfo=MakeGetAbiInfoSA::<extern fn()>::STABLE_ABI;


/////////////

/// Allows one to create the TypeLayout/AbiInfoWrapper for any type `T`,
/// by pretending that it is a primitive type.
/// 
/// Used by the StableAbi derive macro by fields marker as `#[sabi(unsafe_opaque_field)]`.
/// 
/// # Safety
/// 
/// You must ensure that the layout of `T` is compatible through other means.
#[repr(transparent)]
pub struct UnsafeOpaqueField<T>(T);

unsafe impl<T> GetStaticEquivalent_ for UnsafeOpaqueField<T> {
    /// it is fine to use `()` because this type is treated as opaque anyway.
    type StaticEquivalent=();
}
unsafe impl<T> SharedStableAbi for UnsafeOpaqueField<T> {
    type Kind=ValueKind;
    type IsNonZeroType = False;

    const S_LAYOUT: &'static TypeLayout = &TypeLayout::from_params::<Self>(TypeLayoutParams {
        name: "OpaqueField",
        item_info:make_item_info!(),
        data: TLData::Opaque,
        generics: tl_genparams!(;;),
    });
}

/////////////
