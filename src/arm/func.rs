use std::{
    marker::PhantomData,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use super::*;

#[derive(Clone)]
pub struct ArmFunction<Args = (), Ret = ()> {
    meta: PhantomData<(Args, Ret)>,
    arm: WeakArm,
    pub addr: u32,
}

pub trait ArmFn<Args> {
    fn call(&mut self, arm: &Arm, sp: u32);
}

impl<Args, Ret> ArmFunction<Args, Ret> {
    pub fn new(arm: WeakArm, addr: u32) -> Self {
        let meta = PhantomData;
        Self { meta, arm, addr }
    }
}

impl<Args: IntoArmArgs, Ret: FromArm> ArmFunction<Args, Ret> {
    pub fn call(&self, args: Args) -> Result<Ret> {
        let arm = self.arm.upgrade();
        let runner = arm.runner.as_ref();
        let (size, ret) = match Ret::CAN_RET {
            true => (Args::DRET_SIZE, None),
            false => (Args::SRET_SIZE, Some(runner.malloc(Ret::SIZE))),
        };
        let regs = runner.call(self.addr, |sp: u32| {
            let top = AlignDown!(sp - size as u32, 8);
            let ptr = runner.translate(top);
            let stack = unsafe { from_raw_parts_mut(ptr, size) };
            args.into_stack(ret, stack);
            top
        })?;

        if let Some(addr) = ret {
            let ptr = runner.translate(addr);
            let buf = unsafe { from_raw_parts(ptr, Ret::SIZE as usize) };
            let ret = Ret::from_arm(&arm, buf);
            runner.free(addr);
            Ok(ret)
        } else {
            Ok(Ret::from_arm(&arm, &regs))
        }
    }
}

impl<Args, Ret> ArmType for ArmFunction<Args, Ret> {
    const ALIGN: usize = u32::ALIGN;
    const SIZE: usize = u32::SIZE;
}

impl<Args, Ret> IntoArm for ArmFunction<Args, Ret> {
    fn into_arm(self, buf: &mut [u8]) {
        u32::into_arm(self.addr, buf);
    }
}

impl<Args, Ret> FromArm for ArmFunction<Args, Ret> {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        Self::new(arm.weak(), u32::from_arm(arm, buf))
    }
}

// impl<Args> ArmType for dyn ArmFn<Args> {
//     const ALIGN: usize = u32::ALIGN;
//     const SIZE: usize = u32::SIZE;
// }

// impl<Args> IntoArm for dyn ArmFn<Args>
// where
//     Self: Sized,
// {
//     fn into_arm(self, buf: &mut [u8]) {

//         todo!()
//     }
// }

// impl <R, T: Fn(A) -> R, A> ArmType for T {
//     const ALIGN: usize = u32::ALIGN;
//     const SIZE: usize = u32::SIZE;
// }

macro_rules! impl_func {
    (impl $($ident:ident),*) => {
        // impl<R, F: FnMut($( $ident ),*) -> R, $( $ident ),*> ArmType for F
        // where
        //     R: IntoArm,
        //     $( $ident: FromArm ),*
        // {
        //     const ALIGN: usize = u32::ALIGN;
        //     const SIZE: usize = u32::SIZE;
        // }

        impl<R, F: FnMut($( $ident ),*) -> R, $( $ident ),*> ArmFn<($( $ident, )*)> for F
        where
            R: IntoArm,
            $( $ident: FromArm ),*
        {
            #[allow(unused, non_snake_case)]
            fn call(&mut self, arm: &Arm, sp: u32) {
                let mut offset = if R::CAN_RET { 0 } else { 4 };
                $(
                    let $ident = AlignUp!(offset, $ident::ARG_ALIGN);
                    offset = $ident + $ident::ARG_ALIGN_SIZE;
                )*

                let sp = arm.runner.translate(sp);
                let stack = unsafe { from_raw_parts_mut(sp, AlignUp!(offset, 8)) };

                $( let $ident = $ident::from_arm(arm, &stack[$ident..$ident + $ident::SIZE]); )*

                let ret: &mut [u8] = match R::CAN_RET {
                    true => unsafe { from_raw_parts_mut(sp, R::SIZE) },
                    false => {
                        let ret = u32::from_arm(arm, &stack[..u32::SIZE]);
                        let ptr = arm.runner.translate(ret);
                        unsafe { from_raw_parts_mut(ptr, R::SIZE as usize) }
                    }
                };
                self($( $ident ),*).into_arm(ret);
            }
        }

        impl<R, F: FnMut(&Arm, $( $ident ),*) -> R, $( $ident ),*> ArmFn<(&Arm, $( $ident ),*)> for F
        where
            R: IntoArm,
            $( $ident: FromArm ),*
        {
            #[allow(unused, non_snake_case)]
            fn call(&mut self, arm: &Arm, sp: u32) {
                let mut offset = if R::CAN_RET { 0 } else { 4 };
                $(
                    let $ident = AlignUp!(offset, $ident::ARG_ALIGN);
                    offset = $ident + $ident::ARG_ALIGN_SIZE;
                )*

                let sp = arm.runner.translate(sp);
                let stack = unsafe { from_raw_parts_mut(sp, AlignUp!(offset, 8)) };

                $( let $ident = $ident::from_arm(arm, &stack[$ident..$ident + $ident::SIZE]); )*

                let ret: &mut [u8] = match R::CAN_RET {
                    true => unsafe { from_raw_parts_mut(sp, R::SIZE) },
                    false => {
                        let ret = u32::from_arm(arm, &stack[..u32::SIZE]);
                        let ptr = arm.runner.translate(ret);
                        unsafe { from_raw_parts_mut(ptr, R::SIZE as usize) }
                    }
                };
                self(arm, $( $ident ),*).into_arm(ret);
            }
        }
    };
    () => { impl_func!(impl); };
    ($head:ident $(,$ident:ident)*) => {
        impl_func!($($ident),*);
        impl_func!(impl $head $(, $ident )*);
    };
}

impl_func!(
    T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19,
    T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38
);

// impl_func!(impl A, B, C);

// impl_func!(impl A, B, C);

// impl<R, F: FnMut(A, B, C) -> R, A, B, C> ArmFn<(A, B, C)> for F
// where
//     R: IntoArm,
//     A: FromArm,
//     B: FromArm,
//     C: FromArm,
// {
//     #[allow(unused, non_snake_case)]
//     // fn call(&mut self, arm: &Arm, stack: &[u8], mut offset: usize) -> Self::Ret {
//     fn call(&mut self, arm: &Arm, sp: u32) {
//         let sp = arm.runner.translate(sp);
//         let mut offset = if R::CAN_RET { 0 } else { 4 };

//         let A = AlignUp!(offset, A::ARG_ALIGN);
//         offset += A + A::ARG_ALIGN_SIZE;
//         let B = AlignUp!(offset, B::ARG_ALIGN);
//         offset += B + B::ARG_ALIGN_SIZE;
//         let C = AlignUp!(offset, C::ARG_ALIGN);
//         offset += C + C::ARG_ALIGN_SIZE;

//         let stack = unsafe { from_raw_parts_mut(sp, AlignUp!(offset, 8)) };

//         let A = A::from_arm(arm, &stack[A..A + A::SIZE]);
//         let B = B::from_arm(arm, &stack[B..B + B::SIZE]);
//         let C = C::from_arm(arm, &stack[C..C + C::SIZE]);

//         let ret: &mut [u8] = match R::CAN_RET {
//             true => stack,
//             false => {
//                 let ret = u32::from_arm(arm, &stack[0..4]);
//                 let ptr = arm.runner.translate(ret);
//                 unsafe { from_raw_parts_mut(ptr, R::SIZE as usize) }
//             }
//         };
//         self(A, B, C).into_arm(ret);
//     }
// }
// impl<R, F: FnMut(&Arm, A, B, C) -> R, A, B, C> ArmFn<(&Arm, A, B, C)> for F
// where
//     A: FromArm,
//     B: FromArm,
//     C: FromArm,
// {
//     type Ret = R;
//     #[allow(unused, non_snake_case)]
//     // fn call(&mut self, arm: &Arm, stack: &[u8], mut offset: usize) -> Self::Ret {
//     fn call(&mut self, arm: &Arm, sp: u32) -> Self::Ret {
//         offset = AlignUp!(offset, A::ARG_ALIGN);
//         let A = A::from_arm(arm, &stack[offset..offset + A::SIZE]);
//         offset += A::ARG_ALIGN_SIZE;
//         offset = AlignUp!(offset, B::ARG_ALIGN);
//         let B = B::from_arm(arm, &stack[offset..offset + B::SIZE]);
//         offset += B::ARG_ALIGN_SIZE;
//         offset = AlignUp!(offset, C::ARG_ALIGN);
//         let C = C::from_arm(arm, &stack[offset..offset + C::SIZE]);
//         offset += C::ARG_ALIGN_SIZE;
//         self(arm, A, B, C)
//     }
// }

////////////////////////////////////////////////////////////////////

// impl<Args, Ret> ArmType for dyn AnyFn<Args, Ret = Ret> {
//     const ALIGN: u32 = u32::ALIGN;
//     const SIZE: u32 = u32::SIZE;
// }

// impl<Args, Ret> IntoArm for dyn AnyFn<Args, Ret = Ret> {
//     fn into_arm(&self, buf: &mut [u8]) {
//         let a = || ();
//         <dyn AnyFn<(), Ret = ()>>::into_arm(&a, buf);
//         // a.into_arm(buf);

//         let runner: &ArmRunner<'_> = unsafe { &*std::ptr::dangling() };
//         // runner.register(f)
//         // let addr = runner.register(|addr| {
//         //     let args = Args::from_stack(runner, buf, None);
//         //     let ret = self.call(args);
//         //     ret.into_arm(buf);
//         // });

//         todo!()
//     }
// }

// impl<F, Args, Ret> ArmType for F
// where
//     F: AnyFn<Args, Ret = Ret>,
//     Args: IntoArmArgs,
//     Ret: FromArm<'a>,
// {
//     const ALIGN: u32 = u32::ALIGN;
//     const SIZE: u32 = u32::SIZE;
// }

// impl<F, Args, Ret> IntoArm for F
// where
//     F: AnyFn<Args, Ret = Ret>,
//     Args: IntoArmArgs,
//     Ret: FromArm<'a>,
// {
//     fn into_arm(&self, buf: &mut [u8]) {
//         todo!()
//     }
// }
