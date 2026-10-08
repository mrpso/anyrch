use bytes::{Buf, BufMut};

use super::Arm;

mod private {
    pub struct ArmBasicTypeToken;
}

// Rust 类型与 Arm 类型内存布局完全相同的特征
pub unsafe trait ArmSame {}

pub trait ArmType {
    const ALIGN: usize; // Raw Alignment    原始对齐大小
    const SIZE: usize; //  Raw Size         原始数据大小

    // 判断是否为基本类型，并且限制外部实现不能为基本类型
    #[doc(hidden)]
    const BASIC: Option<private::ArmBasicTypeToken> = None;
}

pub trait IntoArm: ArmType + ArmLayout {
    fn into_arm(self, buf: &mut [u8]);
}

pub trait FromArm: ArmType + ArmLayout {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self;
}

pub trait ArmArgsLayout {
    const DRET_SIZE: usize; // 自身原来所有参数的大小
    const SRET_SIZE: usize; // 加隐式返回指针的所有参数大小
}

pub trait IntoArmArgs: ArmArgsLayout {
    fn into_stack(self, ret: Option<u32>, stack: &mut [u8]);
}

pub trait FromArmArgs: ArmArgsLayout {
    fn from_stack(arm: &Arm, stack: &[u8], ret: bool) -> Self;
}

impl<T: ArmType> ArmLayout for T {}
// impl<T: ArmType + ?Sized> ArmLayout for T {}
pub trait ArmLayout: ArmType {
    // 是否为寄存器直接返回类型
    const CAN_RET: bool = Self::BASIC.is_some() || Self::ALIGN_SIZE <= 4;
    // Aligned Size 对齐后的占用内存大小
    const ALIGN_SIZE: usize = AlignUp!(Self::SIZE, Self::ALIGN);
    // 作为函数参数在栈上的对齐大小
    // ((Self::RAW_ALIGN - 1) & 4) + 4
    const ARG_ALIGN: usize = 4 << (Self::ALIGN >= 8) as usize;
    // Argument Alignment 作为参数对齐大小占用内存大小
    const ARG_ALIGN_SIZE: usize = AlignUp!(Self::ALIGN_SIZE, Self::ARG_ALIGN);
}

macro_rules! impl_type {
    ($ty:ty > $size:expr; |$val:ident, $buf:ident| $arg:expr, |$slice:ident| $from:expr) => {
        unsafe impl ArmSame for $ty {}

        impl ArmType for $ty {
            const ALIGN: usize = $size;
            const SIZE: usize = $size;

            #[allow(private_interfaces)]
            const BASIC: Option<private::ArmBasicTypeToken> = Some(private::ArmBasicTypeToken);
        }

        impl IntoArm for $ty {
            #[inline]
            fn into_arm(self, write_buf: &mut [u8]) {
                debug_assert!(write_buf.len() >= <Self as ArmType>::SIZE);
                let ($val, mut $buf) = (self, write_buf);
                $arg;
            }
        }

        impl FromArm for $ty {
            #[inline]
            fn from_arm(_: &Arm, read_buf: &[u8]) -> Self {
                debug_assert!(read_buf.len() >= <Self as ArmType>::SIZE);
                let mut $slice = read_buf;
                $from
            }
        }
    };
}

macro_rules! impl_tuple {
    (impl $($ident:ident),*) => {
        unsafe impl <$( $ident: ArmSame ),*> ArmSame for ($( $ident, )*) {}
        // #[allow(unused)]
        impl <$( $ident: ArmType ),*> ArmType for ($( $ident, )*) {
            const ALIGN: usize = {
                let mut align = 1usize;
                $(
                    if <$ident as ArmType>::ALIGN > align {
                        align = <$ident as ArmType>::ALIGN;
                    }
                )*
                align
            };

            const SIZE: usize = {
                let mut offset = 0usize;
                $(
                    offset = AlignUp!(offset, <$ident as ArmType>::ALIGN);
                    offset += <$ident as ArmLayout>::ALIGN_SIZE;
                )*
                AlignUp!(offset, Self::ALIGN)
            };
        }

        #[allow(non_snake_case, unused)]
        impl <$( $ident: IntoArm ),*> IntoArm for ($( $ident, )*) {
            // #[allow(unused)]
            fn into_arm(self, buf: &mut [u8]) {
                debug_assert!(buf.len() >= <Self as ArmType>::SIZE);
                let ($( $ident, )*) = self;
                let mut offset = 0;
                $(
                    offset = AlignUp!(offset, <$ident as ArmType>::ALIGN);
                    $ident.into_arm(&mut buf[offset..offset + <$ident as ArmType>::SIZE]);
                    offset += <$ident as ArmLayout>::ALIGN_SIZE;
                )*
            }
        }

        #[allow(non_snake_case, unused)]
        impl <$( $ident: FromArm ),*> FromArm for ($( $ident, )*) {
            // #[allow(unused)]
            fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
                debug_assert!(buf.len() >= <Self as ArmType>::SIZE);
                let mut offset = 0;
                $(
                    offset = AlignUp!(offset, <$ident as ArmType>::ALIGN);
                    let $ident = $ident::from_arm(arm, &buf[offset..offset + <$ident as ArmType>::SIZE]);
                    offset += <$ident as ArmLayout>::ALIGN_SIZE;
                )*
                ( $( $ident, )* )
            }
        }
    };
    () => {
        // impl_args!(impl);

        impl ArmType for () {
            const ALIGN: usize = 1;
            const SIZE: usize = 0;
        }

        impl IntoArm for () {
            fn into_arm(self, _: &mut [u8]) {}
        }

        impl FromArm for () {
            fn from_arm(_: &Arm, _: &[u8]) -> Self {}
        }
    };
    ($head:ident $(,$ident:ident)*) => {
        impl_tuple!($($ident),*);
        impl_tuple!(impl $head $(, $ident )*);
    };
}

macro_rules! impl_args {
    (impl $($ident:ident),*) => {
        #[allow(unused)]
        impl <$( $ident: ArmLayout ),*> ArmArgsLayout for ($( $ident, )*) {
            const DRET_SIZE: usize = {
                let mut size = 0;
                $(
                    size = AlignUp!(size, $ident::ARG_ALIGN as usize);
                    size += $ident::ARG_ALIGN_SIZE;
                )*
                size
            };

            const SRET_SIZE: usize = {
                let mut size = 4;
                $(
                    size = AlignUp!(size, $ident::ARG_ALIGN as usize);
                    size += $ident::ARG_ALIGN_SIZE as usize;
                )*
                size
            };
        }

        #[allow(non_snake_case, unused)]
        impl <$( $ident: IntoArm ),*> IntoArmArgs for ($( $ident, )*) {
            fn into_stack(self, ret: Option<u32>, stack: &mut [u8]) {
                let ($( $ident, )*) = self;
                let mut offset = 0usize;
                if let Some(ptr) = ret {
                    ptr.into_arm(&mut stack[..4]);
                    offset = 4;
                }
                $(
                    offset = AlignUp!(offset, $ident::ARG_ALIGN as usize);
                    $ident.into_arm(&mut stack[offset..offset + $ident::SIZE as usize]);
                    offset += $ident::ARG_ALIGN_SIZE as usize;
                )*
            }
        }

        #[allow(non_snake_case, unused)]
        impl <$( $ident: FromArm ),*> FromArmArgs for ($( $ident, )*) {
            fn from_stack(arm: &Arm, stack: &[u8], ret: bool) -> Self {
                let mut offset = ret.then_some(4).unwrap_or_default();
                $(
                    offset = AlignUp!(offset, $ident::ARG_ALIGN as usize);
                    let $ident = $ident::from_arm(arm, &stack[offset..offset + $ident::SIZE as usize]);
                    offset += $ident::ARG_ALIGN_SIZE as usize;
                )*
                ( $( $ident, )* )
            }
        }
    };
    () => {
        impl_args!(impl);

        // #[allow(unused)]
        // impl <T: ArmLayout> ArmArgsLayout for T  {
        //     const DRET_SIZE: usize = T::ARG_ALIGN_SIZE;

        //     const SRET_SIZE: usize = AlignUp!(4, T::ARG_ALIGN) + T::ARG_ALIGN_SIZE;
        // }

        // #[allow(non_snake_case, unused)]
        // impl <T: IntoArm> IntoArmArgs for T {
        //     fn into_stack(self, ret: Option<u32>, stack: &mut [u8]) {
        //         if let Some(ptr) = ret {
        //             ptr.into_arm(&mut stack[..4]);
        //             let offset = AlignUp!(4, T::ARG_ALIGN as usize);
        //             IntoArm::into_arm(self, &mut stack[offset..offset + T::SIZE as usize]);
        //         } else {
        //             IntoArm::into_arm(self, &mut stack[..T::SIZE as usize]);
        //         }
        //     }
        // }

        // #[allow(non_snake_case, unused)]
        // impl <T: FromArm> FromArmArgs for T {
        //     fn from_stack(arm: &Arm, stack: &[u8], ret: bool) -> Self {
        //         let mut offset = ret.then_some(4).unwrap_or_default();
        //         offset = AlignUp!(offset, T::ARG_ALIGN as usize);
        //         T::from_arm(arm, &stack[offset..offset + T::SIZE as usize])
        //     }
        // }
    };
    ($head:ident $(,$ident:ident)*) => {
        impl_args!($($ident),*);
        impl_args!(impl $head $(, $ident )*);
    };
}

// impl<T: ArmType + Copy> ArmType for &T {
//     const ALIGN: usize = T::ALIGN;
//     const SIZE: usize = T::SIZE;

//     const BASIC: Option<private::ArmBasicTypeToken> = T::BASIC;
// }

// impl<T: IntoArm + Copy> IntoArm for &T {
//     fn into_arm(self, buf: &mut [u8]) {
//         T::into_arm(*self, buf);
//     }
// }

impl ArmType for super::Perms {
    const ALIGN: usize = <i32 as ArmType>::ALIGN;
    const SIZE: usize = <i32 as ArmType>::SIZE;
}

impl IntoArm for super::Perms {
    fn into_arm(self, buf: &mut [u8]) {
        <i32 as IntoArm>::into_arm(self.bits(), buf);
    }
}

impl FromArm for super::Perms {
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        Self::from_bits_truncate(<i32 as FromArm>::from_arm(arm, buf))
    }
}

impl<const N: usize, T: ArmType> ArmType for [T; N] {
    const ALIGN: usize = T::ALIGN;
    const SIZE: usize = T::SIZE * N;
}

impl<const N: usize, T: IntoArm> IntoArm for [T; N] {
    #[inline]
    fn into_arm(self, buf: &mut [u8]) {
        debug_assert!(buf.len() >= <Self as ArmLayout>::ALIGN_SIZE);
        for (index, item) in self.into_iter().enumerate() {
            let offset = index * T::ALIGN_SIZE;
            T::into_arm(item, &mut buf[offset..offset + T::SIZE]);
        }
    }
}

impl<const N: usize, T: FromArm> FromArm for [T; N] {
    #[inline]
    fn from_arm(arm: &Arm, buf: &[u8]) -> Self {
        debug_assert!(buf.len() >= <Self as ArmLayout>::ALIGN_SIZE);
        std::array::from_fn(|index| {
            let offset = index * T::ALIGN_SIZE;
            T::from_arm(arm, &buf[offset..offset + T::SIZE])
        })
    }
}

impl_type!( i8  > 1; |val, buf| buf.put_i8(val), |buf| buf.get_i8() );
impl_type!( u8  > 1; |val, buf| buf.put_u8(val), |buf| buf.get_u8() );
impl_type!( i16 > 2; |val, buf| buf.put_i16_le(val), |buf| buf.get_i16_le() );
impl_type!( u16 > 2; |val, buf| buf.put_u16_le(val), |buf| buf.get_u16_le() );
impl_type!( i32 > 4; |val, buf| buf.put_i32_le(val), |buf| buf.get_i32_le() );
impl_type!( u32 > 4; |val, buf| buf.put_u32_le(val), |buf| buf.get_u32_le() );
impl_type!( i64 > 8; |val, buf| buf.put_i64_le(val), |buf| buf.get_i64_le() );
impl_type!( u64 > 8; |val, buf| buf.put_u64_le(val), |buf| buf.get_u64_le() );

impl_type!( bool  > 1; |val, buf| buf.put_u8(val as u8), |buf| buf.get_u8() != 0 );
impl_type!( isize > 4; |val, buf| buf.put_i32_le(val as i32), |buf| buf.get_i32_le() as _ );
impl_type!( usize > 4; |val, buf| buf.put_u32_le(val as u32), |buf| buf.get_u32_le() as _ );

impl_type!( f32  > 4; |val, buf| buf.put_f32_le(val), |buf| buf.get_f32_le() );
impl_type!( f64  > 8; |val, buf| buf.put_f64_le(val), |buf| buf.get_f64_le() );

impl_tuple!(
    T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19,
    T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38
);

impl_args!(
    T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19,
    T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38
);

// impl_tuple!(impl A, B, C);

// impl_args!(A, B, C);
