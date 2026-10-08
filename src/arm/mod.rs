use std::rc::{Rc, Weak};
use std::slice::{from_raw_parts, from_raw_parts_mut};

use super::*;

pub use anyrch_derive::{Arm, ArmType, FromArm, IntoArm};

pub use self::func::*;
pub use self::ptr::*;
pub use self::tys::*;

use self::runner::ArmRunner;

mod func;
mod ptr;
mod tys;
mod str;
mod slice;

mod runner;

// pub enum Arm<'r, 'a> {
//     Ref(&'a ArmRunner<'r>),
//     Own(Box<ArmRunner<'r>>),
// }

pub struct Arm {
    runner: Rc<ArmRunner>,
}

#[derive(Clone)]
pub struct WeakArm {
    runner: Weak<ArmRunner>,
}

impl Arm {
    pub fn create() -> Result<Self> {
        // let runner: Rx<dyn ArmRunner> = cfg_select! {
        //     target_arch = "arm" => Rx!(self::native::Native::create()?),
        //     feature = "unicorn" => Rx!(self::unicorn::Unicorn::create()?),
        //     _ => compile_error!("unicorn feature is required for arm architecture"),
        // };

        let runner = Rc::new(ArmRunner::create()?);
        Ok(Self { runner })
    }

    #[inline(always)]
    pub fn translate<T>(&self, addr: u32) -> *mut T {
        self.runner.translate(addr).cast()
    }

    #[inline(always)]
    pub fn clear_cache(&self, addr: u32, len: u32) {
        self.runner.clear_cache(addr, addr + len);
    }

    fn weak(&self) -> WeakArm {
        WeakArm {
            runner: Rc::downgrade(&self.runner),
        }
    }

    // pub fn null(&self) -> ArmPtr<()> {
    //     ArmPtr::new(self.weak(), 0)
    // }
}

impl WeakArm {
    pub fn upgrade(&self) -> Arm {
        Arm {
            runner: self.runner.upgrade().expect("Arm instance is destroyed"),
        }
    }
}

impl Arm {
    #[inline(always)]
    pub fn memmap(&self, size: usize, perms: Perms) -> ArmPtr<()> {
        ArmPtr::new(self.weak(), self.runner.memmap(size, perms))
    }

    #[inline(always)]
    pub fn munmap(&self, addr: u32, size: usize) {
        self.runner.munmap(addr, size);
    }

    #[inline(always)]
    pub fn malloc(&self, size: usize) -> ArmPtr<()> {
        ArmPtr::new(self.weak(), self.runner.malloc(size))
    }

    #[inline(always)]
    pub fn alloc<T: ArmLayout>(&self) -> ArmPtr<T> {
        // ArmPtr::new(self, self.runner.malloc(T::ALIGN_SIZE))
        self.malloc(T::ALIGN_SIZE).cast()
    }

    pub fn write<T: IntoArm>(&self, val: T) -> ArmPtr<T> {
        let ptr = self.alloc();
        ptr.write(val);
        ptr
    }

    pub fn load<T, R>(&self, code: &[u8], writable: bool) -> ArmFunction<T, R> {
        // let runner = self.runner2();
        let perms = Perms::RX | writable.then_some(Perms::W).unwrap_or_default();
        let addr = self.runner.memmap(code.len(), perms);
        let ptr = self.runner.translate(addr);
        unsafe { ptr.copy_from(code.as_ptr(), code.len()) };
        ArmFunction::new(self.weak(), addr)
    }
}

// pub trait Function : Fn() {

// }

// impl Function {

// }

impl Arm {
    pub fn define<T, F: ArmFn<T> + 'static>(&self, mut f: F) -> ArmPtr<()> {
        let arm = self.weak();
        let addr = self.runner.register(move |sp| f.call(&arm.upgrade(), sp));
        ArmPtr::new(self.weak(), addr)
    }

    #[deprecated = "use define instead"]
    pub fn register<F, Args>(&self, mut f: F) -> ArmPtr<()>
    where
        F: AnyFn<Args> + 'static,
        F::Ret: IntoArm,
        Args: FromArmArgs,
    {
        let arm = self.weak();
        let addr = self.runner.register(move |sp| {
            let arm = arm.upgrade();
            let stack = arm.runner.translate(sp);
            if F::Ret::CAN_RET {
                let stack = unsafe { from_raw_parts_mut(stack, Args::DRET_SIZE) };
                let args = Args::from_stack(&arm, stack, false);
                f.call(args).into_arm(stack);
            } else {
                let stack = unsafe { from_raw_parts(stack, Args::SRET_SIZE) };
                let ptr = arm
                    .runner
                    .translate(bytes::Buf::get_u32_le(&mut &stack[0..4]));
                let ret = f.call(Args::from_stack(&arm, stack, true));
                ret.into_arm(unsafe { from_raw_parts_mut(ptr, F::Ret::SIZE as usize) });
            };
        });
        ArmPtr::new(self.weak(), addr)
    }
}

// impl<'r, 'a> Arm<'r, 'a> {
//     fn runner2(&self) -> &'a ArmRunner<'r> {
//         // let ptr = match self {
//         //     Self::Ref(runner) => std::ptr::from_ref(*runner),
//         //     Self::Own(runner) => std::ptr::from_ref(runner.as_ref()),
//         // };
//         // unsafe { ptr.cast_mut().as_mut_unchecked() }
//         match self {
//             Self::Ref(runner) => *runner,
//             Self::Own(runner) => {
//                 let ptr = std::ptr::from_ref(runner.as_ref());
//                 unsafe { ptr.cast_mut().as_mut_unchecked() }
//             },
//         }
//     }
// }

// impl ArmType for &Arm {
//     const ALIGN: u32 = 0;
//     const SIZE: u32 = 0;
// }

// impl FromArm for &Arm {
//     fn from_arm(arm: &Arm, _: &[u8]) -> Self {
//         arm
//     }
// }

// impl ArmType for &Arm<'_> {
//     const ALIGN: u32 = 0;
//     const SIZE: u32 = 0;
// }

// impl<'a, 'r> FromArm<'a, 'r> for &Arm<'r>
// {
//     fn from_arm(arm: &'a Arm<'r>, _: &[u8]) -> Self {
//         arm
//     }
// }
