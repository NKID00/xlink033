#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, feature(impl_trait_in_assoc_type))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(coroutines)]
#![feature(coroutine_trait)]

pub use coro_macros::yields;

use core::{
    cell::UnsafeCell,
    default::Default,
    mem::MaybeUninit,
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

pub trait Coro:
    Coroutine<
        <Self as Coro>::Arg,
        Yield = <<Self as Coro>::Coro as Coroutine<<Self as Coro>::Arg>>::Yield,
        Return = <<Self as Coro>::Coro as Coroutine<<Self as Coro>::Arg>>::Return,
    >
{
    type Arg;
    type Coro: Coroutine<Self::Arg>;
    fn new_coro() -> Self::Coro;
}

pub trait WrappingCoro: Coro + Default
where
    Self::Coro:
        Coroutine<<Self as Coro>::Arg, Yield = Self::YieldReturn, Return = Self::YieldReturn>,
{
    type YieldReturn;
    fn wrapping_resume(self: Pin<&mut Self>, arg: <Self as Coro>::Arg) -> Self::YieldReturn;
}

impl<T: Coro + Default, U> WrappingCoro for T
where
    T::Coro: Coroutine<<T as Coro>::Arg, Yield = U, Return = U>,
{
    type YieldReturn = U;
    #[inline]
    fn wrapping_resume(mut self: Pin<&mut Self>, arg: <Self as Coro>::Arg) -> U {
        match self.as_mut().resume(arg) {
            CoroutineState::Yielded(v) => v,
            CoroutineState::Complete(v) => {
                self.set(Default::default());
                v
            }
        }
    }
}

pub struct UnsafeStatic<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> UnsafeStatic<T> {
    #[inline]
    pub const fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    /// Safety: must not use after initialization, or use [`replace`] instead
    #[inline]
    pub unsafe fn init(&'static self, val: T) {
        unsafe { (*self.0.get()).write(val) };
    }

    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn replace(&'static self, val: T) {
        unsafe { (*self.0.get()).assume_init_drop() };
        unsafe { self.init(val) };
    }

    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn as_ref(&'static self) -> &'static T {
        unsafe { (*self.0.get()).assume_init_ref() }
    }

    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn as_mut(&'static self) -> &'static mut T {
        unsafe { (*self.0.get()).assume_init_mut() }
    }

    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn as_pin(&'static self) -> Pin<&'static mut T> {
        Pin::static_mut(unsafe { self.as_mut() })
    }
}

unsafe impl<T> Sync for UnsafeStatic<T> {}

impl<T: Default> UnsafeStatic<T> {
    /// Safety: must not use after initialization, or use [`replace`] instead
    #[inline]
    pub unsafe fn init_default(&'static self) {
        unsafe { self.init(Default::default()) };
    }

    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn reset(&'static self) {
        unsafe { self.replace(Default::default()) };
    }
}

impl<T, U> UnsafeStatic<T>
where
    T: WrappingCoro<YieldReturn = U>,
    T::Coro: Coroutine<<T as Coro>::Arg, Yield = U, Return = U>,
{
    /// Safety: must not use before initialization
    #[inline]
    pub unsafe fn wrapping_resume(self: &'static Self, arg: <T as Coro>::Arg) -> U {
        Pin::static_mut(unsafe { self.as_mut() }).wrapping_resume(arg)
    }
}

#[cfg(test)]
extern crate self as coro;

#[cfg(test)]
mod test {
    use super::*;

    use core::{assert_matches, ops::CoroutineState, pin::Pin};

    #[coro::yields(Result<u32, ()>)]
    fn TestStateMachine(a: u32) -> Option<u32> {
        let b = yield Ok(a + 1);
        return Some(b * 2);
    }

    #[test]
    fn test_coro() {
        let mut machine = TestStateMachine::new();
        assert!(matches!(
            Pin::new(&mut machine).resume(1),
            CoroutineState::Yielded(Ok(2))
        ));
        assert!(matches!(
            Pin::new(&mut machine).resume(3),
            CoroutineState::Complete(Some(6))
        ));
    }
}

// static STATE_M: StaticCoroutine = StaticCoroutine::new();

// #[coro::coroutine]
// fn aaa() {

// }

// struct StaticCoroutine(UnsafeCell<Option<<StaticCoroutine as CoroutineTrait>::CoroutineT>>);

// impl StaticCoroutine {
//     pub const fn new() -> Self {
//         Self(UnsafeCell::new(None))
//     }

//     pub unsafe fn reset(&'static self) {
//         unsafe {
//             self.0
//                 .as_mut_unchecked()
//                 .insert(StaticCoroutine::new_coroutine())
//         };
//     }

//     pub unsafe fn as_mut(
//         &'static self,
//     ) -> &'static mut <StaticCoroutine as CoroutineTrait>::CoroutineT {
//         unsafe { self.0.as_mut_unchecked().as_mut().unwrap_unchecked() }
//     }
// }

// impl CoroutineTrait for StaticCoroutine {
//     type CoroutineT = impl Coroutine<(), Yield = (), Return = ()>;
//     // #[rust_analyzer::skip]
//     // #[define_opaque(CoroutineT)]
//     fn new_coroutine() -> Self::CoroutineT {
//         #[coroutine]
//         || {
//             yield;
//         }
//     }
// }

// unsafe impl Sync for StaticCoroutine {}
