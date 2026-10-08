use super::{Arm, ArmType, FromArm, IntoArm, WeakArm};

pub struct ArmPtr<T> {
    pub addr: u32,
    arm: WeakArm,
    free: bool,
    meta: std::marker::PhantomData<T>,
}

impl<T> ArmPtr<T> {
    pub fn is_null(&self) -> bool {
        self.addr == 0
    }

    pub fn raw(&self) -> *mut u8 {
        self.arm.upgrade().translate(self.addr)
    }

    pub fn as_slice<'a>(&self, len: usize) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.raw(), len) }
    }

    pub fn as_mut_slice<'a>(&self, len: usize) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.raw(), len) }
    }

    pub fn cast<U>(self) -> ArmPtr<U> {
        unsafe { std::mem::transmute(self) }
    }

    pub fn free(mut self) -> Self {
        self.free = true;
        self
    }
}

impl<T> ArmPtr<T> {
    pub fn new(arm: WeakArm, addr: u32) -> Self {
        Self {
            meta: std::marker::PhantomData,
            arm,
            addr,
            free: false,
        }
    }
}

impl<T: IntoArm> ArmPtr<T> {
    pub fn write(&self, val: T) {
        if self.addr != 0 {
            let buf = self.as_mut_slice(T::SIZE);
            <T as IntoArm>::into_arm(val, buf);
        };
    }
}

impl<T: FromArm> ArmPtr<T> {
    #[inline(always)]
    pub fn read(&self) -> T {
        <T as FromArm>::from_arm(&self.arm.upgrade(), self.as_slice(T::SIZE as usize))
    }
}

impl<T: IntoArm + FromArm> ArmPtr<T> {
    pub fn map<F: FnOnce(&Arm, &mut T)>(&self, f: F) {
        let mut val = self.read();
        f(&self.arm.upgrade(), &mut val);
        self.write(val);
    }
}

impl<T> ArmType for ArmPtr<T> {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl<T> IntoArm for ArmPtr<T> {
    fn into_arm(self, buf: &mut [u8]) {
        <u32 as IntoArm>::into_arm(self.addr, buf);
    }
}

impl<T> FromArm for ArmPtr<T> {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        Self::new(arm.weak(), <u32 as FromArm>::from_arm(arm, buf))
    }
}

impl<T> Clone for ArmPtr<T> {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr.clone(),
            arm: self.arm.clone(),
            free: false,
            meta: self.meta.clone(),
        }
    }
}

impl<T> Drop for ArmPtr<T> {
    fn drop(&mut self) {
        if self.free && !self.is_null() {
            self.arm.upgrade().runner.free(self.addr);
        }
    }
}

impl<T> ArmType for Option<ArmPtr<T>> {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl<T> IntoArm for Option<ArmPtr<T>> {
    fn into_arm(self, buf: &mut [u8]) {
        let addr = self.map(|ptr| ptr.addr).unwrap_or_default();
        <u32 as IntoArm>::into_arm(addr, buf);
    }
}

impl<T> FromArm for Option<ArmPtr<T>> {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        let addr = <u32 as FromArm>::from_arm(arm, buf);
        (addr != 0).then(|| ArmPtr::new(arm.weak(), addr))
    }
}

impl<T> ArmType for *mut T {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl<T> FromArm for *mut T {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        arm.translate::<T>(<u32 as FromArm>::from_arm(arm, buf))
    }
}
