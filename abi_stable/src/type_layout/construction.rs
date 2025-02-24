use super::*;


////////////////////////////////////////////////////////////////////////////////


/// The parameters for `TypeLayout::from_params`.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TypeLayoutParams {
    /// The name of the type,without generic parameters.
    pub name: &'static str,
    /// Information about where the type was declared,
    /// generally created with `make_item_info!()`.
    pub item_info:ItemInfo,
    /// The definition of the type.
    pub data: TLData,
    /// The generic parameters of the type,
    /// generally constructed with the `tl_genparams` macro.
    pub generics: GenericParams,
}


#[doc(hidden)]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct _private_TypeLayoutDerive {
    pub name: &'static str,
    pub item_info:ItemInfo,
    pub data: TLData,
    pub generics: GenericParams,
    pub phantom_fields: &'static [TLField],
    pub tag:Tag,
    pub mod_refl_mode:ModReflMode,
    pub repr_attr:ReprAttr,
}


/// Information about where a type was declared.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq,StableAbi)]
pub struct ItemInfo{
    /// The package where the type was defined,and the version string.
    /// With the `package;version_number` format.
    pub package_and_version:StaticStr,
    /// The line in the file where the type was defined.
    pub line:u32,
    /// The full path to the module where the type was defined,
    /// including the package name
    pub mod_path:ModPath,
}

impl ItemInfo{
    #[doc(hidden)]
    pub const fn new(
        package_and_version:&'static str,
        line:u32,
        mod_path:ModPath,
    )->Self{
        Self{
            package_and_version: StaticStr::new(package_and_version),
            line,
            mod_path,
        }
    }

    /// Constructs an ItemInfo for a std primitive
    pub const fn primitive()->Self{
        Self{
            package_and_version: StaticStr::new("std;1.0.0"),
            line:0,
            mod_path:ModPath::Prelude,
        }
    }

    /// Constructs an ItemInfo for an std type with a path.
    pub const fn std_type_in(path:&'static str)->Self{
        Self{
            package_and_version: StaticStr::new("std;1.0.0"),
            line:0,
            mod_path:ModPath::inside(path),
        }
    }

    /// Constructs an ItemInfo for a type in a package and the path to its module.
    ///
    /// `package_and_version` must be formatted like this:`package_name;major.minor.patch`
    ///
    /// `mod_path` must include the crate name.
    pub const fn package_and_mod(package_and_version:&'static str,mod_path:&'static str)->Self{
        Self{
            package_and_version:StaticStr::new(package_and_version),
            line:0,
            mod_path:ModPath::inside(mod_path),
        }
    }
}


impl ItemInfo{
    pub fn package_and_version(&self)->(&'static str,&'static str){
        let pav=self.package_and_version.as_str();
        match pav.find(';') {
            Some(separator)=>{
                (&pav[..separator],&pav[(separator+1)..])
            }
            None=>{
                (&pav[..],"")
            }
        }
    }
}