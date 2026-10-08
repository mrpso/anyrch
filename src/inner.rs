// #[allow(unused_macros)]
// macro_rules! Rx {
//     ($value: expr) => {
//         $crate::Rx::new(std::sync::Arc::new(parking_lot::ReentrantMutex::new(
//             $value,
//         )))
//     };
// }

#[macro_export]
macro_rules! AlignUp {
    ($val:expr, $align:expr) => {{
        let (val, align) = (($val), ($align));
        debug_assert!(align > 0 && (align & (align - 1)) == 0);
        (val + align - 1) & !(align - 1)
    }};
}

#[macro_export]
macro_rules! AlignDown {
    ($val:expr, $align:expr) => {{
        let (val, align) = (($val), ($align));
        debug_assert!(align > 0 && (align & (align - 1)) == 0);
        val & !(align - 1)
    }};
}
