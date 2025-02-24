use super::{
    *,
    vtable::InterfaceBound,
};
    
use std::{
    fmt,
    ops::{Deref,DerefMut},
};

use core_extensions::SelfOps;

use crate::{
    abi_stability::SharedStableAbi,
    erased_types::c_functions::adapt_std_fmt,
    sabi_types::MaybeCmp,
    std_types::{RBox,UTypeId},
    pointer_trait::{
        TransmuteElement,
        GetPointerKind,PK_SmartPointer,PK_Reference,
    },
    type_level::{
        unerasability::{TU_Unerasable,TU_Opaque},
        impl_enum::{Implemented,Unimplemented},
        trait_marker,
    },
    utils::{transmute_reference,transmute_mut_reference},
};


/**
`RObject` implements ffi-safe trait objects,for a minimal selection of traits.

The main use of `RObject<_>` is as the default backend for `#[sabi_trait]` 
generated trait objects.

# Construction

To construct an `RObject<_>` directly (not as part of a trait object) 
you can call one of these methods:

- from_ptr
    Can be constructed from a pointer of a value.Cannot unerase the RObject afterwards.

- from_ptr_unerasable:
    Can be constructed from a pointer of a value.Requires a `'static` value.

- from_value:
    Can be constructed from the value directly.Cannot unerase the RObject afterwards.

- from_value_unerasable
    Can be constructed from the value directly.Requires a `'static` value.

# Trait object

`RObject<'borrow,Pointer<()>,Interface,VTable>` 
can be used as a trait object for any combination of 
the traits listed bellow.

These are the traits:

- Send

- Sync

- Debug

- Clone

# Deconstruction

`RObject<_>` can then be unwrapped into a concrete type,
within the same dynamic library/executable that constructed it,
using these (fallible) conversion methods:

- sabi_into_any_unerased:Unwraps into a pointer to `T`.Requires `T:'static`.

- sabi_as_any_unerased:Unwraps into a `&T`.Requires `T:'static`.

- sabi_as_any_unerased_mut:Unwraps into a `&mut T`.Requires `T:'static`.

`RObject` can only be converted back if it was created 
using a `RObject::*_unerased` function.


*/
#[repr(C)]
#[derive(StableAbi)]
#[sabi(
    not_stableabi(V),
    bound="V:SharedStableAbi",
    bound="I:InterfaceBound",
    tag="<I as InterfaceBound>::TAG",
)]
pub struct RObject<'lt,P,I,V>{
    vtable:StaticRef<V>,
    is_reborrowed:bool,
    ptr: ManuallyDrop<P>,
    _marker:PhantomData<Tuple2<&'lt (),I>>,
}


macro_rules! impl_from_ptr_method {
    ( 
        $( #[$attr:meta] )*
        method_name=$method_name:ident,
        requires_any=$requires_any:ty 
    ) => (
        $( #[$attr] )*
        pub fn $method_name<'lt,OrigPtr,Params>(
            ptr:OrigPtr,
        )-> RObject<'lt,P,I,V>
        where 
            OrigPtr:TransmuteElement<(),TransmutedPtr=P>+'lt,
            P:Deref<Target=()>,
            OrigPtr::Target:Sized+'lt,
            I:GetVTable<
                $requires_any,
                OrigPtr::Target,
                OrigPtr::TransmutedPtr,
                OrigPtr,
                Params,
                VTable=V
            >,
            I:GetRObjectVTable<$requires_any,OrigPtr::Target,OrigPtr::TransmutedPtr,OrigPtr>
        {
            RObject{
                vtable:I::get_vtable(),
                is_reborrowed:false,
                ptr:unsafe{
                    let ptr=TransmuteElement::<()>::transmute_element(ptr,PhantomData);
                    ManuallyDrop::new(ptr)
                },
                _marker:PhantomData,
            }
        }

    )
}

impl<P,I,V> RObject<'_,P,I,V>{
    impl_from_ptr_method!{
        /**
Creates a trait object from a pointer to a type that must implement the 
trait that `I` requires.

The constructed trait object cannot be converted back to the original type.

        */
        method_name=from_ptr,
        requires_any=TU_Opaque 
    }
    impl_from_ptr_method!{
        /**
Creates a trait object from a pointer to a type that must implement the 
trait that `I` requires.

The constructed trait object can be converted back to the original type with 
the `sabi_*_unerased` methods (RObject reserves `sabi` as a prefix for its own methods).

        */
        method_name=from_ptr_unerasable,
        requires_any=TU_Unerasable 
    }
}

impl<I,V> RObject<'_,RBox<()>,I,V>{
/**
Creates a trait object from a type that must implement the trait that `I` requires.

The constructed trait object cannot be converted back to the original type.
*/
    pub fn from_value<'lt,T,Params>(
        value:T,
    )-> RObject<'lt,RBox<()>,I,V>
    where 
        T:'lt,
        I:GetVTable<TU_Opaque,T,RBox<()>,RBox<T>,Params,VTable=V>,
        I:GetRObjectVTable<TU_Opaque,T,RBox<()>,RBox<T>>
    {
        Self::from_ptr::<_,Params>(RBox::new(value))
    }

/**
Creates a trait object from a type that must implement the trait that `I` requires.

The constructed trait object can be converted back to the original type with 
the `sabi_*_unerased` methods (RObject reserves `sabi` as a prefix for its own methods).
*/
    pub fn from_value_unerasable<'lt,T,Params>(
        value:T,
    )-> RObject<'lt,RBox<()>,I,V>
    where 
        T:'lt,
        I:GetVTable<TU_Unerasable,T,RBox<()>,RBox<T>,Params,VTable=V>,
        I:GetRObjectVTable<TU_Unerasable,T,RBox<()>,RBox<T>>
    {
        Self::from_ptr_unerasable::<_,Params>(RBox::new(value))
    }
}



mod clone_impl{
    pub trait CloneImpl<PtrKind>{
        fn clone_impl(&self) -> Self;
    }
}
use self::clone_impl::CloneImpl;


/// This impl is for smart pointers.
impl<'lt,P, I,V> CloneImpl<PK_SmartPointer> for RObject<'lt,P,I,V>
where
    P: Deref,
    I: InterfaceType<Clone = Implemented<trait_marker::Clone>>,
{
    fn clone_impl(&self) -> Self {
        let ptr=unsafe{
            self.sabi_robject_vtable()._sabi_clone().unwrap()(&self.ptr)
        };
        Self{
            vtable:self.vtable,
            is_reborrowed:self.is_reborrowed,
            ptr:ManuallyDrop::new(ptr),
            _marker:PhantomData,
        }
    }
}

/// This impl is for references.
impl<'lt,P, I,V> CloneImpl<PK_Reference> for RObject<'lt,P,I,V>
where
    P: Deref+Copy,
    I: InterfaceType<Clone = Implemented<trait_marker::Clone>>,
{
    fn clone_impl(&self) -> Self {
        Self{
            vtable:self.vtable,
            is_reborrowed:self.is_reborrowed,
            ptr:ManuallyDrop::new(*self.ptr),
            _marker:PhantomData,
        }
    }
}

/**
Clone is implemented for references and smart pointers,
using `GetPointerKind` to decide whether `P` is a smart pointer or a reference.

RObject does not implement Clone if P==`&mut ()` :


```compile_fail
use abi_stable::{
    sabi_trait::prelude::*,
    trait_object_test::*,
    std_types::*,
};

let mut object=RSomething_TO::<_,()>::from_value(RBox::new(10_u32),TU_Opaque);
let borrow=object.reborrow_mut();
let _=borrow.clone();
```

*/
impl<'lt,P, I,V> Clone for RObject<'lt,P,I,V>
where
    P: Deref+GetPointerKind,
    I: InterfaceType,
    Self:CloneImpl<<P as GetPointerKind>::Kind>,
{
    fn clone(&self) -> Self {
        self.clone_impl()
    }
}


impl<'lt,P,I,V> Debug for RObject<'lt,P,I,V> 
where
    P: Deref<Target=()>,
    I: InterfaceType<Debug = Implemented<trait_marker::Debug>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe{
            adapt_std_fmt::<ErasedObject>(
                self.sabi_erased_ref(), 
                self.sabi_robject_vtable()._sabi_debug().unwrap(), 
                f
            )
        }
    }
}


unsafe impl<'lt,P,I,V> Send for RObject<'lt,P,I,V> 
where 
    I:InterfaceType<Send = Implemented<trait_marker::Send>>,
{}

unsafe impl<'lt,P,I,V> Sync for RObject<'lt,P,I,V> 
where 
    I:InterfaceType<Sync = Implemented<trait_marker::Sync>>,
{}


impl<'lt,P,I,V> RObject<'lt,P,I,V>{
/**

Constructs an RObject from a pointer and an extra vtable.

This is mostly intended to be called by `#[sabi_trait]` derived trait objects.

# Safety

These are the requirements for the caller:

- `P` must be a pointer to the type that the vtable functions 
    take as the first parameter.

- The vtable must not come from a reborrowed RObject
    (created using RObject::reborrow or RObject::reborrow_mut).

- The vtable must be the `<SomeVTableName>` of a struct declared with 
    `#[derive(StableAbi)]``#[sabi(kind(Prefix(prefix_struct="<SomeVTableName>")))]`.

- The vtable must have `StaticRef<RObjectVtable<..>>` 
    as its first declared field

*/
    pub unsafe fn with_vtable<OrigPtr>(
        ptr:OrigPtr,
        vtable:StaticRef<V>,
    )-> RObject<'lt,P,I,V>
    where 
        OrigPtr:TransmuteElement<(),TransmutedPtr=P>+'lt,
        OrigPtr::Target:Sized+'lt,
        P:Deref<Target=()>,
    {
        RObject{
            vtable,
            is_reborrowed:false,
            ptr:ManuallyDrop::new( ptr.transmute_element(<()>::T) ),
            _marker:PhantomData,
        }
    }
}

impl<'lt,P,I,V> RObject<'lt,P,I,V>{
    /// The uid in the vtable has to be the same as the one for T,
    /// otherwise it was not created from that T in the library that 
    /// declared the trait object.
    fn sabi_check_same_utypeid<T>(&self) -> Result<(), UneraseError<()>>
    where
        T:'static,
    {
        let expected_typeid=self.sabi_robject_vtable()._sabi_type_id().get();
        let actual_typeid=UTypeId::new::<T>();
        if expected_typeid == MaybeCmp::Just(actual_typeid) {
            Ok(())
        } else {
            Err(UneraseError {
                robject:(),
                expected_typeid,
                actual_typeid,
            })
        }
    }

    /// Attempts to unerase this trait object into the pointer it was constructed with.
    ///
    /// # Errors
    ///
    /// This will return an error in any of these conditions:
    ///
    /// - It is called in a dynamic library/binary outside
    /// the one from which this RObject was constructed.
    ///
    /// - The RObject was constructed using the `from_(value|ptr)` method
    ///
    /// - `T` is not the concrete type this `RObject<_>` was constructed with.
    ///
    pub fn sabi_into_any_unerased<T>(self) -> Result<P::TransmutedPtr, UneraseError<Self>>
    where
        T:'static,
        P: Deref<Target=()>+TransmuteElement<T>,
    {
        check_unerased!(self,self.sabi_check_same_utypeid::<T>());
        unsafe {
            let this=ManuallyDrop::new(self);
            Ok(ptr::read(&*this.ptr).transmute_element(T::T)) 
        }
    }

    /// Attempts to unerase this trait object into a reference of 
    /// the value was constructed with.
    ///
    /// # Errors
    ///
    /// This will return an error in any of these conditions:
    ///
    /// - It is called in a dynamic library/binary outside
    /// the one from which this RObject was constructed.
    ///
    /// - The RObject was constructed using the `from_(value|ptr)` method
    ///
    /// - `T` is not the concrete type this `RObject<_>` was constructed with.
    ///
    pub fn sabi_as_any_unerased<T>(&self) -> Result<&T, UneraseError<&Self>>
    where
        T:'static,
        P:Deref<Target=()>+TransmuteElement<T>,
    {
        check_unerased!(self,self.sabi_check_same_utypeid::<T>());
        unsafe { 
            Ok(transmute_reference::<(),T>(&**self.ptr))
        }
    }

    /// Attempts to unerase this trait object into a mutable reference of 
    /// the value was constructed with.
    ///
    /// # Errors
    ///
    /// This will return an error in any of these conditions:
    ///
    /// - It is called in a dynamic library/binary outside
    /// the one from which this RObject was constructed.
    ///
    /// - The RObject was constructed using the `frfrom_(value|ptr)_*` method
    ///
    /// - `T` is not the concrete type this `RObject<_>` was constructed with.
    ///
    pub fn sabi_as_any_unerased_mut<T>(&mut self) -> Result<&mut T, UneraseError<&mut Self>>
    where
        T:'static,
        P:DerefMut<Target=()>+TransmuteElement<T>,
    {
        check_unerased!(self,self.sabi_check_same_utypeid::<T>());
        unsafe { 
            Ok(transmute_mut_reference::<(),T>(&mut **self.ptr))
        }
    }

}


mod private_struct {
    pub struct PrivStruct;
}
use self::private_struct::PrivStruct;


/// This is used to make sure that reborrowing does not change 
/// the Send-ness or Sync-ness of the pointer.
pub trait ReborrowBounds<SendNess,SyncNess>{}

// If it's reborrowing,it must have either both Sync+Send or neither.
impl ReborrowBounds<Unimplemented<trait_marker::Send>,Unimplemented<trait_marker::Sync>> 
for PrivStruct 
{}

impl ReborrowBounds<Implemented<trait_marker::Send>,Implemented<trait_marker::Sync>> 
for PrivStruct 
{}


impl<'lt,P,I,V> RObject<'lt,P,I,V>
where 
    I:InterfaceType
{
    /// Creates a shared reborrow of this RObject.
    ///
    pub fn reborrow<'re>(&'re self)->RObject<'lt,&'re (),I,V> 
    where
        P:Deref<Target=()>,
        PrivStruct:ReborrowBounds<I::Send,I::Sync>,
    {
        // Reborrowing will break if I add extra functions that operate on `P`.
        RObject{
            vtable:self.vtable,
            is_reborrowed:true,
            ptr:ManuallyDrop::new(&**self.ptr),
            _marker:PhantomData,
        }
    }

    /// Creates a mutable reborrow of this RObject.
    ///
    /// The reborrowed RObject cannot use these methods:
    ///
    /// - RObject::clone
    /// 
    pub fn reborrow_mut<'re>(&'re mut self)->RObject<'lt,&'re mut (),I,V> 
    where
        P:DerefMut<Target=()>,
        PrivStruct:ReborrowBounds<I::Send,I::Sync>,
    {
        // Reborrowing will break if I add extra functions that operate on `P`.
        RObject {
            vtable: self.vtable,
            is_reborrowed:true,
            ptr: ManuallyDrop::new(&mut **self.ptr),
            _marker:PhantomData,
        }
    }
}




impl<'lt,P,I,V> RObject<'lt,P,I,V>{

    #[inline]
    pub fn sabi_et_vtable<'a>(&self)->&'a V{
        self.vtable.get()
    }

    /// The vtable common to all `#[sabi_trait]` generated trait objects.
    #[inline]
    pub fn sabi_robject_vtable<'a>(&self)->&'a RObjectVtable<(),P,I>{
        unsafe{ 
            let vtable=&*(self.vtable.get() as *const V as *const BaseVtable<(),P,I>);
            vtable._sabi_vtable().get()
        }
    }

    #[inline]
    fn sabi_into_erased_ptr(self)->ManuallyDrop<P>{
        let mut __this= ManuallyDrop::new(self);
        unsafe{ ptr::read(&mut __this.ptr) }
    }

    #[inline]
    pub fn sabi_erased_ref(&self)->&ErasedObject<()>
    where
        P: __DerefTrait<Target=()>
    {
        unsafe{&*((&**self.ptr) as *const () as *const ErasedObject<()>)}
    }
    
    #[inline]
    pub fn sabi_erased_mut(&mut self)->&mut ErasedObject<()>
    where
        P: __DerefMutTrait<Target=()>
    {
        unsafe{&mut *((&mut **self.ptr) as *mut () as *mut ErasedObject<()>)}
    }

    #[inline]
    pub fn sabi_with_value<F,R>(self,f:F)->R
    where 
        P: OwnedPointer<Target=()>,
        F:FnOnce(MovePtr<'_,()>)->R,
    {
        OwnedPointer::with_move_ptr(self.sabi_into_erased_ptr(),f)
    }
}

impl<P,I,V> Drop for RObject<'_,P,I,V>{
    fn drop(&mut self){
        // This condition is necessary because if the RObject was reborrowed,
        // the destructor function would take a different pointer type.
        if !self.is_reborrowed{
            let destructor=self.sabi_robject_vtable()._sabi_drop();
            unsafe{
                destructor(&mut self.ptr);
            }
        }
    }
}





//////////////////////////////////////////////////////////////////

/// Error for `RObject<_>` being unerased into the wrong type
/// with one of the `*unerased*` methods.
#[derive(Copy, Clone)]
pub struct UneraseError<T> {
    robject:T,
    expected_typeid:MaybeCmp<UTypeId>,
    actual_typeid:UTypeId,
}


impl<T> UneraseError<T>{
    fn map<F,U>(self,f:F)->UneraseError<U>
    where F:FnOnce(T)->U
    {
        UneraseError{
            robject        :f(self.robject),
            expected_typeid:self.expected_typeid,
            actual_typeid  :self.actual_typeid,
        }
    }

    /// Extracts the RObject,to handle the failure to unerase it.
    #[must_use]
    pub fn into_inner(self)->T{
        self.robject
    }
}


impl<D> fmt::Debug for UneraseError<D>{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UneraseError")
            .field("dyn_trait",&"<not shown>")
            .field("expected_typeid",&self.expected_typeid)
            .field("actual_typeid",&self.actual_typeid)
            .finish()
    }
}

impl<D> fmt::Display for UneraseError<D>{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<D> ::std::error::Error for UneraseError<D> {}

//////////////////////////////////////////////////////////////////
