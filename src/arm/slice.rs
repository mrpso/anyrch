use std::net::{Ipv4Addr, SocketAddrV4};

use super::{Arm, ArmSame, ArmType, FromArm};

impl<T: ArmSame> ArmType for &[T] {
    const ALIGN: usize = <usize as ArmType>::ALIGN;
    const SIZE: usize = <usize as ArmType>::SIZE * 2;
}

impl<T: ArmSame> FromArm for &[T] {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        let addr = <u32 as FromArm>::from_arm(arm, &buf[..4]);
        let len = <usize as FromArm>::from_arm(arm, &buf[4..8]);
        unsafe { std::slice::from_raw_parts(arm.translate(addr), len) }
    }
}

impl<T: ArmSame> ArmType for &mut [T] {
    const ALIGN: usize = <usize as ArmType>::ALIGN;
    const SIZE: usize = <usize as ArmType>::SIZE * 2;
}

impl<T: ArmSame> FromArm for &mut [T] {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        let addr = <u32 as FromArm>::from_arm(arm, &buf[..4]);
        let len = <usize as FromArm>::from_arm(arm, &buf[4..8]);
        unsafe { std::slice::from_raw_parts_mut(arm.translate(addr), len) }
    }
}

impl ArmType for Ipv4Addr {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE;
}

impl FromArm for Ipv4Addr {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        Self::from_bits(<u32 as FromArm>::from_arm(arm, buf))
    }
}

impl ArmType for SocketAddrV4 {
    const ALIGN: usize = <u32 as ArmType>::ALIGN;
    const SIZE: usize = <u32 as ArmType>::SIZE + <u16 as ArmType>::SIZE;
}

impl FromArm for SocketAddrV4 {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        let ip = Ipv4Addr::from_arm(arm, &buf[..4]);
        let port = <u16 as FromArm>::from_arm(arm, &buf[4..6]);
        Self::new(ip, port)
    }
}
