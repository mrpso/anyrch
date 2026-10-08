use std::ffi::CStr;

pub trait FromPtr {
    fn from_ptr(raw: *mut ()) -> Self;
}

impl FromPtr for &CStr {
    fn from_ptr(raw: *mut ()) -> Self {
        unsafe { CStr::from_ptr(raw.cast()) }
    }
}

// impl FromPtr for Cow<'_, str> {
//     fn from_ptr(raw: *mut ()) -> Self {
//         <&CStr as FromPtr>::from_ptr(raw).to_string_lossy()
//     }
// }

// impl FromPtr for String {
//     fn from_ptr(raw: *mut ()) -> Self {
//         <Cow<'_, str> as FromPtr>::from_ptr(raw).to_string()
//     }
// }
