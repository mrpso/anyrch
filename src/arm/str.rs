use std::ffi::CStr;

use super::{Arm, ArmType, FromArm};

impl ArmType for &CStr {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl FromArm for &CStr {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        <Option<&CStr> as FromArm>::from_arm(arm, buf).unwrap()
    }
}

impl ArmType for Option<&CStr> {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl FromArm for Option<&CStr> {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        let addr = <u32 as FromArm>::from_arm(arm, buf);
        (addr != 0).then(|| unsafe { CStr::from_ptr(arm.translate(addr)) })
    }
}
