/*!
Types and submodules for doing runtime reflection.
*/

#[cfg(all(test,not(feature="only_new_tests")))]
pub mod tests{
    pub mod derive_reflection;
}

/**
Implementation details of the sabi_extract tool.

This is here so that its tests run among other abi_stable tests.
*/
#[doc(hidden)]
pub mod export_module;

/// Whether this is a module whose definition can be reflected on at runtime,
///
/// Module reflection only allows accessing public fields.
#[repr(u8)]
#[derive(Debug,Copy,Clone,PartialEq,Eq,StableAbi)]
pub enum ModReflMode{
    /// For modules that are reflected on at runtime.
    ///
    /// This is the default for all types.
    Module,
    /// For types whose layout can't be iterated over.
    ///
    /// If one uses `#[sabi(module_reflection(Opaque))]` 
    Opaque,
    /// Delegates the layout to some other type,this is generally for references.
    DelegateDeref{
        /// To which phantom field this delegates to.
        ///
        /// If one uses `#[sabi(module_reflection(Deref))]` 
        /// a phantom field with the type of `<Self as Deref>::Target`
        /// will be created,and this will refer to it.
        phantom_field_index:usize,
    },
}


impl Default for ModReflMode{
    fn default()->Self{
        ModReflMode::Opaque
    }
}
