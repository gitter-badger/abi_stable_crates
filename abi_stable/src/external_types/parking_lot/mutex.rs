use std::{
    cell::UnsafeCell,
    fmt::{self,Debug,Display},
    ops::{Deref,DerefMut},
    marker::PhantomData,
    mem,
};

use parking_lot::{RawMutex};
use lock_api::{
    RawMutex as RawMutexTrait,
    RawMutexTimed,
};

use super::{RAW_LOCK_SIZE,UnsafeOveralignedField};

use crate::{
    StableAbi,
    marker_type::UnsyncUnsend,
    prefix_type::{PrefixTypeTrait,WithMetadata},
    std_types::*,
};


///////////////////////////////////////////////////////////////////////////////

type OpaqueMutex=
    UnsafeOveralignedField<RawMutex,[u8;OM_PADDING]>;

const OM_PADDING:usize=RAW_LOCK_SIZE-mem::size_of::<RawMutex>();

const OPAQUE_MUTEX:OpaqueMutex=
    OpaqueMutex::new(<RawMutex as RawMutexTrait>::INIT,[0u8;OM_PADDING]);

#[allow(dead_code)]
fn assert_mutex_size(){
    let _assert_size:[();RAW_LOCK_SIZE-mem::size_of::<OpaqueMutex>()];
    let _assert_size:[();mem::size_of::<OpaqueMutex>()-RAW_LOCK_SIZE];
}

/**
A mutual exclusion lock that allows dynamic mutable borrows of shared data.

# Poisoning 

As opposed to the standard library version of this type,
this mutex type does not use poisoning,
simply unlocking the lock when a panic happens.

*/
#[repr(C)]
#[derive(StableAbi)]
pub struct RMutex<T>{
    raw_mutex:OpaqueMutex,
    data:UnsafeCell<T>,
    vtable:*const VTable,
}


/**
A mutex guard,which allows mutable access to the data inside the mutex.

When dropped this will unlock the mutex.
*/
#[repr(transparent)]
#[derive(StableAbi)]
#[sabi(bound="T:'a")]
#[must_use]
pub struct RMutexGuard<'a, T> {
    rmutex: &'a RMutex<T>,
    _marker: PhantomData<Tuple2<&'a mut T, UnsyncUnsend>>,
}



///////////////////////////////////////////////////////////////////////////////


impl<T> RMutex<T>{
    /// Constructs a mutex,wrapping `value`.
    pub const fn new(value:T)->Self{
        Self{
            raw_mutex:OPAQUE_MUTEX,
            data:UnsafeCell::new(value),
            vtable:VTable::VTABLE.as_prefix_raw(),
        }
    }

    #[inline]
    fn vtable(&self)->&'static VTable{
        unsafe{&*self.vtable}
    }

    #[inline]
    fn make_guard(&self)->RMutexGuard<'_,T>{
        RMutexGuard{
            rmutex:self,
            _marker:PhantomData
        }
    }

    /// Unwraps this mutex into its wrapped data.
    #[inline]
    pub fn into_inner(self)->T{
        self.data.into_inner()
    }

    /// Gets a mutable reference to its wrapped data.
    ///
    /// This does not require any locking,since it takes `self` mutably.
    #[inline]
    pub fn get_mut(&mut self)->RMutexGuard<'_,T>{
        self.make_guard()
    }

    /**
Acquires a mutex,blocking the current thread until it can.

This function returns a guard which releases the mutex when it is dropped.

Trying to lock the mutex in the same theread that holds the lock will cause a deadlock.
    */
    #[inline]
    pub fn lock(&self)->RMutexGuard<'_,T>{
        self.vtable().lock()(&self.raw_mutex);
        self.make_guard()
    }
    /**
Attemps to acquire a mutex.

Returns the mutex guard if the mutex can be immediately acquired,otherwise returns RNone.
*/    
    #[inline]
    pub fn try_lock(&self) -> ROption<RMutexGuard<'_,T>>{
        if self.vtable().try_lock()(&self.raw_mutex) {
            RSome(self.make_guard())
        }else{
            RNone
        }
    }
    
/**
Attempts to acquire a mutex for the timeout duration.

Once the timeout is reached,this will return None,
otherwise it will return the mutex guard.
*/
    #[inline]
    pub fn try_lock_for(&self, timeout: RDuration) -> ROption<RMutexGuard<'_,T>>{
        if self.vtable().try_lock_for()(&self.raw_mutex,timeout) {
            RSome(self.make_guard())
        }else{
            RNone
        }
    }
}

unsafe impl<T:Send> Send for RMutex<T>
where RawMutex:Send
{}

unsafe impl<T:Send> Sync for RMutex<T>
where RawMutex:Sync
{}

///////////////////////////////////////////////////////////////////////////////


impl<'a,T> Display for RMutexGuard<'a, T> 
where
    T:Display
{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        Display::fmt(&**self,f)
    }
}


impl<'a,T> Debug for RMutexGuard<'a, T> 
where
    T:Debug
{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        Debug::fmt(&**self,f)
    }
}


impl<'a,T> Deref for RMutexGuard<'a, T> {
    type Target=T;

    fn deref(&self)->&T{
        unsafe{ &*self.rmutex.data.get() }
    }
}


impl<'a,T> DerefMut for RMutexGuard<'a, T> {
    fn deref_mut(&mut self)->&mut T{
        unsafe{ &mut *self.rmutex.data.get() }
    }
}

impl<'a,T> Drop for RMutexGuard<'a, T> {
    fn drop(&mut self){
        let vtable=self.rmutex.vtable();
        vtable.unlock()(&self.rmutex.raw_mutex);
    }
}



///////////////////////////////////////////////////////////////////////////////


#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_struct="VTable")))]
#[sabi(missing_field(panic))]
struct VTableVal{
    lock:extern "C" fn(this:&OpaqueMutex),
    try_lock:extern "C" fn(this:&OpaqueMutex) -> bool,
    unlock:extern "C" fn(this:&OpaqueMutex),
    #[sabi(last_prefix_field)]
    try_lock_for:extern "C" fn(this:&OpaqueMutex, timeout: RDuration) -> bool,
}

impl VTable{
    // The VTABLE for this type in this executable/library
    const VTABLE: &'static WithMetadata<VTableVal> = 
        &WithMetadata::new(
            PrefixTypeTrait::METADATA,
            VTableVal{
                lock,
                try_lock,
                unlock,
                try_lock_for,
            }
        );
}


extern "C" fn lock(this:&OpaqueMutex){
    extern_fn_panic_handling!{
        this.value.lock();
    }
}
extern "C" fn try_lock(this:&OpaqueMutex) -> bool{
    extern_fn_panic_handling!{
        this.value.try_lock()       
    }
}
extern "C" fn unlock(this:&OpaqueMutex){
    extern_fn_panic_handling!{
        this.value.unlock();
    }
}
extern "C" fn try_lock_for(this:&OpaqueMutex, timeout: RDuration) -> bool{
    extern_fn_panic_handling!{
        this.value.try_lock_for(timeout.into())
    }
}


///////////////////////////////////////////////////////////////////////////////




#[cfg(all(test,not(feature="only_new_tests")))]
mod tests{
    use super::*;

    use std::{
        thread,
        time::Duration,
    };

    use crossbeam_utils::thread::scope as scoped_thread;

    use crate::test_utils::check_formatting_equivalence;

    #[test]
    fn get_mut(){
        let mut mutex:RMutex<usize>=RMutex::new(0);
        *mutex.lock()+=100;
        *mutex.get_mut()+=100;
        *mutex.lock()+=100;
        assert_eq!(*mutex.lock(), 300);
    }


    #[test]
    fn into_inner(){
        let mutex:RMutex<usize>=RMutex::new(0);
        *mutex.lock()+=100;
        assert_eq!(mutex.into_inner(), 100);
    }

    #[test]
    fn debug_display(){
        let str_="\nhello\rhello\rhello\n";
        let mutex=RMutex::new(str_);
        let guard=mutex.lock();

        check_formatting_equivalence(&guard,str_);
    }

    #[test]
    fn lock(){
        static MUTEX:RMutex<usize>=RMutex::new(0);

        scoped_thread(|scope|{
            for _ in 0..8 {
                scope.spawn(move|_|{
                    for _ in 0..0x1000 {
                        *MUTEX.lock()+=1;
                    }
                });
            }
        }).unwrap();

        assert_eq!(*MUTEX.lock(),0x8000);
    }

    #[test]
    fn try_lock(){
        static MUTEX:RMutex<usize>=RMutex::new(0);

        scoped_thread(|scope|{
            for _ in 0..8 {
                scope.spawn(move|_|{
                    for _ in 0..0x1000 {
                        loop {
                            if let RSome(mut guard)=MUTEX.try_lock() {
                                *guard+=1;
                                break;
                            }
                        }
                    }
                });
            }
        }).unwrap();

        scoped_thread(|scope|{
            let _guard=MUTEX.lock();
            scope.spawn(move|_|{
                assert_eq!(MUTEX.try_lock().map(drop), RNone);
            });
            thread::sleep(Duration::from_millis(100));
        }).unwrap();

        assert_eq!(*MUTEX.lock(),0x8000);
    }

    #[test]
    fn try_lock_for(){
        static MUTEX:RMutex<usize>=RMutex::new(0);

        scoped_thread(|scope|{
            for _ in 0..8 {
                scope.spawn(move|_|{
                    for i in 0..0x1000 {
                        let wait_for=RDuration::new(0,(i+1)*500_000);
                        loop {
                            if let RSome(mut guard)=MUTEX.try_lock_for(wait_for) {
                                *guard+=1;
                                break;
                            }
                        }
                    }
                });
            }
        }).unwrap();


        scoped_thread(|scope|{
            let _guard=MUTEX.lock();
            scope.spawn(move|_|{
                assert_eq!(MUTEX.try_lock_for(RDuration::new(0,100_000)).map(drop), RNone);
            });
            thread::sleep(Duration::from_millis(100));
        }).unwrap();


        assert_eq!(*MUTEX.lock(),0x8000);
    }

}