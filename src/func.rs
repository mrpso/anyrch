// pub trait OwnFn<Args> {
//     type Ret;

//     fn call(&self, args: Args, ret: &mut [u8]);
// }

// pub trait RefFn<Ref, Args> {
//     type Ret;

//     fn call(&self, this: Ref, args: Args, ret: &mut [u8]);
// }

// pub trait AnyFn2 {
//     type Args;
//     type Ret;

//     fn call(&self, args: Self::Args) -> Self::Ret;
// }

pub trait AnyFn<Args> {
    type Ret;

    fn call(&mut self, args: Args) -> Self::Ret;
}

macro_rules! impl_func {
    (impl $($ident:ident),*) => {
        impl<F: FnMut($( $ident ),*) -> R + 'static, R, $( $ident ),*> AnyFn<( $( $ident, )* )> for F {
            type Ret = F::Output;

            #[allow(unused, non_snake_case)]
            fn call(&mut self, args: ( $( $ident, )* )) -> Self::Ret {
                let ($( $ident, )*) = args;
                self($( $ident, )*)
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
