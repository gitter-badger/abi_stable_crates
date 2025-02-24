/*!
Contains many ffi-safe equivalents of standard library types.
The vast majority of them can be converted to and from std equivalents.

For ffi-safe equivalents/wrappers of types outside the standard library go to 
the [external_types module](../external_types/index.html)

*/

pub mod arc;
pub mod boxed;
pub mod cmp_ordering;
pub mod cow;
//pub mod old_cow;
pub mod option;
pub mod map;
pub mod range;
pub mod result;
pub mod slice_mut;
pub mod slices;
pub mod static_slice;
pub mod static_str;
pub mod std_error;
pub mod std_io;
pub mod str;
pub mod string;
pub mod time;
pub mod tuple;
pub mod utypeid;
pub mod vec;


/**
There are equivalents to types in the std::sync module in abi_stable::external_types.

The `sync::{Mutex,RwLock,Once}` equivalents are declared in 
`abi_stable::external_types::parking_lot`

The `mpsc` equivalents are declared in 
`abi_stable::external_types::crossbeam_channel`,
this is enabled by default with the `channels`/`crossbeam-channel` cargo feature.

*/
pub mod sync{}


#[doc(inline)]
pub use self::{
    arc::RArc,
    boxed::RBox,
    cmp_ordering::RCmpOrdering,
    cow::RCow,
    map::RHashMap,
    option::{RNone, ROption, RSome},
    result::{RErr, ROk, RResult},
    slice_mut::RSliceMut,
    slices::RSlice,
    std_error::{RBoxError,SendRBoxError, UnsyncRBoxError},
    std_io::{RIoError,RSeekFrom, RIoErrorKind},
    str::RStr,
    string::RString,
    time::RDuration,
    tuple::{Tuple1,Tuple2, Tuple3, Tuple4},
    vec::RVec,
    utypeid::UTypeId,
    static_str::StaticStr,
    static_slice::StaticSlice,
};