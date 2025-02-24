/*!

For Rust-to-Rust ffi,
with a focus on creating libraries loaded at program startup,
and with load-time type-checking.

This library allows defining Rust libraries that can be loaded at runtime,
even if they were built with a different Rust version than the crate that depends on it.

These are some usecases for this library:
    
- Converting a Rust dependency tree from compiling statically into a single binary,
    into one binary (and potentially) many dynamic libraries,
    allowing separate re-compilation on changes.

- Creating a plugin system (without support for unloading).
    
# Features

Currently this library has these features:

- Features the [`#[sabi_trait]`](./docs/sabi_trait_attribute/index.html)
    attribute,for creating ffi-safe trait objects.

- [ffi-safe equivalent of some trait objects](./erased_types/dyn_trait/struct.DynTrait.html) 
    for any combination of them.

- Provides ffi-safe alternatives/wrappers for many standard library types,
    in the [`std_types`](./std_types/index.html) module.

- Provides ffi-safe wrappers for some crates,
    in the [`external_types`](./external_types/index.html) module.

- Provides the [`StableAbi`](./abi_stability/stable_abi_trait/trait.StableAbi.html) 
    trait for asserting that types are ffi-safe.

- [Features for building extensible modules and vtables](./docs/prefix_types/index.html),
    without breaking ABI compatibility.

- [Supports ffi-safe nonexhaustive enums](./docs/sabi_nonexhaustive/index.html),
    wrapped in [`NonExhaustive<>`](./nonexhaustive_enum/nonexhaustive/struct.NonExhaustive.html).

- Checking at load-time that the types in the dynamic library have the expected layout,
    allowing for semver compatible changes while checking the layout of types.

- Provides the [`StableAbi` derive macro](./docs/stable_abi_derive/index.html)
    to both assert that the type is ffi compatible,
    and to get the layout of the type at load-time to check that it is still compatible.

# Examples

For **examples** of using `abi_stable` you can look at the readme,
or for the crates in the examples directory in the repository for this crate.

To run the examples generally you'll have to build the `*_impl` crate,
then run the `*_user` crate (all `*_user` crates should have a help message and a readme.md).

# Glossary

`interface crate`:the crate that declares the public functions and types that 
are necessary to load the library at runtime.

`ìmplementation crate`:A crate that implements all the functions in the interface crate.

`user crate`:A crate that depends on an `interface crate` and 
loads 1 or more `ìmplementation crate`s for it.

`module`:refers to a struct of function pointers and other static values.
The root module implement the RootModule trait.
These are declared in the `interface crate`,exported in the `implementation crate`,
and loaded in the `user crate`.

# Rust-to-Rust FFI types.

Types must implement StableAbi to be safely passed through the FFI boundary,
which can be done using the StableAbi derive macro.

These are the kinds of types passed through FFI:

- Value kind:<br>
    The layout of types passed by value must not change in a minor version.
    This is the default kind when deriving StableAbi.

- [Nonexhaustive enums](./docs/sabi_nonexhaustive/index.html):<br>
    Enums wrapped inside 
    [`NonExhaustive<>`](./nonexhaustive_enum/nonexhaustive/struct.NonExhaustive.html),
    which can add variants in minor versions of the library.

- Opaque kind:<br>
    Types wrapped in `DynTrait<SomePointer<()>,Interface>`,
    whose layout can change in any version of the library,
    and can only be unwrapped back to the original type in the dynamic library/binary 
    that created it.

- [Trait objects](./docs/sabi_trait_attribute/index.html):<br>
    Trait object-like types generated using `#[sabi_trait]`,
    which erase the type of the value they wrap,implements the methods of the trait,
    and can be unwrapped back to the original type in the dynamic library/binary 
    that created it (if it was constructed to be unerasable and implements Any).

- [Prefix kind](./docs/prefix_types/index.html):<br>
    Types only accessible through shared references,
    most commonly vtables and modules,
    which can be extended in minor versions while staying ABI compatible.
    by adding fields at the end.

# Extra documentation

- [Unsafe code guidelines](./docs/unsafe_code_guidelines/index.html):<br>
    Describes how to write unsafe code ,relating to this library.

# Macros (derive and attribute)

- [sabi_trait attribute macro](./docs/sabi_trait_attribute/index.html):<br>
    For generating ffi-safe trait objects.

- [StableAbi derive macro](./docs/stable_abi_derive/index.html):<br>
    For asserting abi-stability of a type,
    and obtaining the layout of the type at runtime.

- [Nonexhaustive enums](./docs/sabi_nonexhaustive/index.html):<br>
    Details for how to declare nonexhaustive enums.

- [Prefix-types (using the StableAbi derive macro)
  ](./docs/prefix_types/index.html):<br>
    The method by which *vtables* and *modules* are implemented,
    allowing extending them in minor versions of a library.

*/

#![allow(unused_unsafe)]
#![deny(unused_must_use)]
#![warn(rust_2018_idioms)]

#[allow(unused_imports)]
#[cfg(test)]
use abi_stable_shared::file_span;

#[macro_use]
extern crate serde_derive;

#[macro_use(StableAbi)]
extern crate abi_stable_derive;

extern crate self as abi_stable;


#[doc(inline)]
pub use abi_stable_derive::{
    StableAbi,
    GetStaticEquivalent,
};

#[doc(inline)]
pub use abi_stable_derive::{
    export_root_module,
    impl_InterfaceType,
    sabi_trait,
    sabi_extern_fn,
};

#[macro_use]
mod impls;


#[macro_use]
mod macros;


#[cfg(test)]
#[macro_use]
mod test_macros;

#[cfg(test)]
#[macro_use]
mod test_utils;

#[cfg(test)]
mod misc_tests;

#[macro_use]
pub mod utils;

#[macro_use]
pub mod const_utils;

#[macro_use]
pub mod traits;


#[macro_use]
pub mod abi_stability;
// pub mod cabi_type;
// pub mod as_proxy;
#[macro_use]
pub mod erased_types;
pub mod external_types;
// pub mod immovable_wrapper;
#[macro_use]
pub mod library;
pub mod marker_type;
mod multikey_map;
pub mod nonexhaustive_enum;
pub mod pointer_trait;
pub mod prefix_type;
pub mod type_layout;
pub mod inline_storage;



#[doc(hidden)]
pub mod derive_macro_reexports;
pub mod std_types;
pub mod sabi_types;



pub mod reflection;
pub mod type_level;

pub mod docs;

pub mod sabi_trait;


/// The header used to identify the version number of abi_stable
/// that a dynamic libraries uses.
pub static LIB_HEADER:library::AbiHeader=library::AbiHeader::VALUE;


/// Miscelaneous items re-exported from core_extensions.
pub mod reexports{
    pub use core_extensions::SelfOps;
}

/*
I am using this static as the `identity` of this dynamic library/executable,
this assumes that private static variables don't get merged between 
Rust dynamic libraries that have a different global allocator.

If the address of this is the same among dynamic libraries that have *different* 
allocators,please create an issue for this.
*/
use std::sync::atomic::AtomicUsize;
static EXECUTABLE_IDENTITY: AtomicUsize = AtomicUsize::new(1);

#[doc(inline)]
pub use crate::{
    abi_stability::StableAbi,
    erased_types::{DynTrait,ImplType, InterfaceType},
};



#[doc(hidden)]
pub mod globals{
    use crate::{
        abi_stability::{
            abi_checking::{check_layout_compatibility_for_ffi},
            stable_abi_trait::AbiInfoWrapper,
        },
        sabi_types::LateStaticRef,
        std_types::{RResult,RBoxError},
        utils::leak_value,
    };

    #[repr(C)]
    #[derive(StableAbi)]
    pub struct Globals{
        pub layout_checking:
            extern fn(&'static AbiInfoWrapper,&'static AbiInfoWrapper) -> RResult<(), RBoxError> ,
    }

    impl Globals{
        pub fn new()->&'static Self{
            leak_value(Globals{
                layout_checking:check_layout_compatibility_for_ffi,
            })
        }
    }

    pub(crate)static GLOBALS:LateStaticRef<Globals>=LateStaticRef::new();

    #[inline(never)]
    pub fn initialized_globals()->&'static Globals{
        GLOBALS.init(|| Globals::new() )
    }


    #[inline(never)]
    pub extern fn initialize_globals_with(globs:&'static Globals){
        GLOBALS.init(|| globs );
    }
}

