use super::*;

use crate::{
    erased_types::FormattingMode,
    type_level::{
        unerasability::{GetUTID},
        impl_enum::{Implemented,Unimplemented,IsImplemented},
        trait_marker,
    },
    sabi_types::ReturnValueEquality,
    std_types::{UTypeId,RResult,RString},
};

/// Gets the vtable of a trait object.
///
/// # Safety
///
/// The vtable of the type implementing this trait must have
/// `StaticRef<RObjectVtable<_Self,ErasedPtr,I>>` as its first declared field.
pub unsafe trait GetVTable<IA,_Self,ErasedPtr,OrigPtr,Params>{
    type VTable;
    fn get_vtable()->StaticRef<Self::VTable>;
}

/// Gets an `RObjectVtable<_Self,ErasedPtr,TO>`,
/// which is stored as the first field of all generated trait object vtables.
pub unsafe trait GetRObjectVTable<IA,_Self,ErasedPtr,OrigPtr>:Sized+InterfaceType{
    const ROBJECT_VTABLE:StaticRef<RObjectVtable<_Self,ErasedPtr,Self>>;
}




unsafe impl<IA,_Self,ErasedPtr,OrigPtr,I> 
    GetRObjectVTable<IA,_Self,ErasedPtr,OrigPtr> for I
where
    I:AreTraitsImpld<IA,_Self,ErasedPtr,OrigPtr>+InterfaceType,
{
    const ROBJECT_VTABLE:StaticRef<RObjectVtable<_Self,ErasedPtr,Self>>={
        let prefix=GetRObjectVTableHelper::<IA,_Self,ErasedPtr,OrigPtr,I>::TMP_VTABLE;
        WithMetadata::staticref_as_prefix(prefix)
    };
}


#[doc(hidden)]
pub trait AreTraitsImpld<IA,_Self,ErasedPtr,OrigPtr>:Sized {
    const VTABLE_VAL:RObjectVtableVal<_Self,ErasedPtr,Self>;
}

impl<IA,_Self,ErasedPtr,OrigPtr,I> AreTraitsImpld<IA,_Self,ErasedPtr,OrigPtr> for I
where 
    I:InterfaceType,
    I::Sync:RequiresSync<_Self,ErasedPtr,OrigPtr>,
    I::Send:RequiresSend<_Self,ErasedPtr,OrigPtr>,
    I::Clone:InitCloneField<_Self,ErasedPtr,OrigPtr>,
    I::Debug:InitDebugField<_Self,ErasedPtr,OrigPtr>,
    IA:GetUTID<_Self>,
{
    const VTABLE_VAL:RObjectVtableVal<_Self,ErasedPtr,I>=
        RObjectVtableVal{
            _sabi_tys:PhantomData,
            _sabi_type_id:<IA as GetUTID<_Self>>::UID,
            _sabi_drop:c_functions::drop_pointer_impl::<OrigPtr,ErasedPtr>,
            _sabi_clone:<I::Clone as InitCloneField<_Self,ErasedPtr,OrigPtr>>::VALUE,
            _sabi_debug:<I::Debug as InitDebugField<_Self,ErasedPtr,OrigPtr>>::VALUE,
        };
}

// A dummy type used to get around a compiler limitation WRT associated constants in traits.
#[doc(hidden)]
struct GetRObjectVTableHelper<IA,_Self,ErasedPtr,OrigPtr,I>(IA,_Self,ErasedPtr,OrigPtr,I);

impl<IA,_Self,ErasedPtr,OrigPtr,I> 
    GetRObjectVTableHelper<IA,_Self,ErasedPtr,OrigPtr,I>
where 
    I:AreTraitsImpld<IA,_Self,ErasedPtr,OrigPtr>,
{
    const __TMP_0:*const WithMetadata<RObjectVtableVal<_Self,ErasedPtr,I>>={
        &__sabi_re::WithMetadata::new(
            PrefixTypeTrait::METADATA,
            I::VTABLE_VAL,
        )
    };
    const TMP_VTABLE:StaticRef<WithMetadata<RObjectVtableVal<_Self,ErasedPtr,I>>>=unsafe{
        StaticRef::from_raw(Self::__TMP_0)
    };


}


/// The vtable for RObject,which all  `#[trait_object]` derived trait objects contain.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_struct="RObjectVtable")))]
#[sabi(missing_field(default))]
pub struct RObjectVtableVal<_Self,ErasedPtr,I>{
    pub _sabi_tys:PhantomData<extern "C" fn(_Self,ErasedPtr,I)>,
    
    pub _sabi_type_id:ReturnValueEquality<MaybeCmp<UTypeId>>,

    #[sabi(last_prefix_field)]
    pub _sabi_drop :unsafe extern "C" fn(this:&mut ErasedPtr),
    pub _sabi_clone:Option<unsafe extern "C" fn(this:&ErasedPtr)->ErasedPtr>,
    pub _sabi_debug:Option<
        unsafe extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>
    >,
}



/// The common prefix of all `#[trait_object]` derived vtables,
/// with `RObjectVtable` as its first field.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(
    bound="I:InterfaceBound",
    tag="<I as InterfaceBound>::TAG",
    kind(Prefix(prefix_struct="BaseVtable")),
)]
pub(super)struct BaseVtableVal<_Self,ErasedPtr,I>{
    pub _sabi_tys:PhantomData<unsafe extern "C" fn(_Self,ErasedPtr,I)>,

    #[sabi(last_prefix_field)]
    pub _sabi_vtable:StaticRef<RObjectVtable<_Self,ErasedPtr,I>>,
}

use self::trait_bounds::*;
pub mod trait_bounds{
    use super::*;

    macro_rules! declare_conditional_marker {
        (
            type $selector:ident;
            trait $trait_name:ident[$self_:ident,$ErasedPtr:ident,$OrigPtr:ident]
            where [ $($where_preds:tt)* ]
        ) => (
            pub trait $trait_name<$self_,$ErasedPtr,$OrigPtr>{}

            impl<$self_,$ErasedPtr,$OrigPtr> $trait_name<$self_,$ErasedPtr,$OrigPtr> 
            for Unimplemented<trait_marker::$selector>
            {}
            
            impl<$self_,$ErasedPtr,$OrigPtr> $trait_name<$self_,ErasedPtr,$OrigPtr> 
            for Implemented<trait_marker::$selector>
            where
                $($where_preds)*
            {}
        )
    }

    macro_rules! declare_field_initalizer {
        (
            type $selector:ident;
            trait $trait_name:ident[$self_:ident,$ErasedPtr:ident,$OrigPtr:ident]
            where [ $($where_preds:tt)* ]
            type=$field_ty:ty,
            value=$field_value:expr,
        ) => (
            pub trait $trait_name<$self_,$ErasedPtr,$OrigPtr>{
                const VALUE:Option<$field_ty>;
            }

            impl<$self_,$ErasedPtr,$OrigPtr> $trait_name<$self_,$ErasedPtr,$OrigPtr> 
            for Unimplemented<trait_marker::$selector>
            {
                const VALUE:Option<$field_ty>=None;
            }

            impl<$self_,$ErasedPtr,$OrigPtr> $trait_name<$self_,ErasedPtr,$OrigPtr> 
            for Implemented<trait_marker::$selector>
            where
                $($where_preds)*
            {
                const VALUE:Option<$field_ty>=Some($field_value);
            }
        )
    }


    declare_conditional_marker!{
        type Send;
        trait RequiresSend[_Self,ErasedPtr,OrigPtr]
        where [ _Self:Send,OrigPtr:Send ]
    }

    declare_conditional_marker!{
        type Sync;
        trait RequiresSync[_Self,ErasedPtr,OrigPtr]
        where [ _Self:Sync,OrigPtr:Sync ]
    }

    declare_field_initalizer!{
        type Clone;
        trait InitCloneField[_Self,ErasedPtr,OrigPtr]
        where [ OrigPtr:Clone ]
        type=unsafe extern "C" fn(this:&ErasedPtr)->ErasedPtr,
        value=c_functions::clone_pointer_impl::<OrigPtr,ErasedPtr>,
    }
    declare_field_initalizer!{
        type Debug;
        trait InitDebugField[_Self,ErasedPtr,OrigPtr]
        where [ _Self:Debug ]
        type=unsafe extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>,
        value=c_functions::debug_impl::<_Self>,
    }




}

macro_rules! declare_InterfaceBound {
    (
        auto_traits=[ $( $auto_trait:ident ),* ]
        required_traits=[ $( $required_traits:ident ),* ]
    ) => (

        #[allow(non_upper_case_globals)]
        pub trait InterfaceBound:InterfaceType{
            const TAG:Tag;
            $(const $auto_trait:bool;)*
            $(const $required_traits:bool;)*
        }

        #[allow(non_upper_case_globals)]
        impl<I> InterfaceBound for I
        where 
            I:InterfaceType,
            $(I::$auto_trait:IsImplemented,)*
            $(I::$required_traits:IsImplemented,)*
        {
            const TAG:Tag={
                const fn str_if(cond:bool,s:&'static str)->Tag{
                    [ Tag::null(), Tag::str(s) ][cond as usize]
                }

                tag!{{
                    "auto traits"=>tag![[
                        $(  
                            str_if(
                                <I::$auto_trait as IsImplemented>::VALUE,
                                stringify!($auto_trait)
                            ),
                        )*
                    ]],
                    "required traits"=>tag!{{
                        $(  
                            str_if(
                                <I::$required_traits as IsImplemented>::VALUE,
                                stringify!($required_traits)
                            ),
                        )*
                    }}
                }}
            };

            $(const $auto_trait:bool=<I::$auto_trait as IsImplemented>::VALUE;)*
            $(const $required_traits:bool=<I::$required_traits as IsImplemented>::VALUE;)*
        }
    )
}

declare_InterfaceBound!{
    auto_traits=[ Sync,Send ]
    required_traits=[ Clone,Debug ]
}
