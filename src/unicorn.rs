#![allow(unused)]

use std::ffi::*;

use unicorn_engine::{
    Unicorn,
    unicorn_const::uc_error,
};

pub trait UnicornExtend {
    fn reg_read_batch<const N: usize, R: Into<i32>>(
        &mut self,
        regs: [R; N],
    ) -> Result<[u64; N], uc_error>;

    fn reg_write_batch<const N: usize, R: Into<i32>>(
        &mut self,
        pairs: [(R, u64); N],
    ) -> Result<(), uc_error>;

    fn remove_cache(&self, addr: u64, end: u64) -> Result<(), uc_error>;
}

// No input and output arguments.
const UC_CTL_IO_NONE: u32 = 0;
// Only input arguments for a write operation.
const UC_CTL_IO_WRITE: u32 = 1;
// Only output arguments for a read operation.
const UC_CTL_IO_READ: u32 = 2;

#[repr(C)]
enum ControlType {
    // Current mode.
    // Read: @args = (int*)
    UcMode = 0,
    // Curent page size.
    // Write: @args = (uint32_t)
    // Read: @args = (uint32_t*)
    UcPageSize,
    // Current arch.
    // Read: @args = (int*)
    UcArch,
    // Current timeout.
    // Read: @args = (uint64_t*)
    UcTimeout,
    // Enable multiple exits.
    // Without this control, reading/setting exits won't work.
    // This is for API backward compatibility.
    // Write: @args = (int)
    UcUseExits,
    // The number of current exits.
    // Read: @args = (size_t*)
    UcExitsCnt,
    // Current exits.
    // Write: @args = (uint64_t* exits, size_t len)
    //        @len = UC_CTL_UC_EXITS_CNT
    // Read: @args = (uint64_t* exits, size_t len)
    //       @len = UC_CTL_UC_EXITS_CNT
    UcExits,

    // Set the cpu model of uc.
    // Note this option can only be set before any Unicorn
    // API is called except for uc_open.
    // Write: @args = (int)
    // Read:  @args = (int*)
    CpuModel,
    // Request a tb cache at a specific address
    // Read: @args = (uint64_t, uc_tb*)
    TbRequestCache,
    // Invalidate a tb cache at a specific address
    // Write: @args = (uint64_t, uint64_t)
    TbRemoveCache,
    // Invalidate all translation blocks.
    // No arguments.
    TbFlush,
}

unsafe extern "C" {
    unsafe fn uc_reg_read_batch(
        handle: *mut c_void,
        ids: *const c_int,
        vals: *const *mut c_void,
        count: c_int,
    ) -> uc_error;

    unsafe fn uc_reg_write_batch(
        handle: *mut c_void,
        ids: *const c_int,
        vals: *const *const c_void,
        count: c_int,
    ) -> uc_error;

    unsafe fn uc_ctl(uc: *mut c_void, control: u32, ...) -> uc_error;
}

impl<'a, T> UnicornExtend for Unicorn<'a, T> {
    fn reg_read_batch<const N: usize, R: Into<i32>>(
        &mut self,
        regs: [R; N],
    ) -> Result<[u64; N], uc_error> {
        let mut values = [0u64; N];
        let vals = values.each_mut().map(|x| (&raw mut *x).cast());
        let ids = regs.map(|reg| reg.into());
        let (ids, vals, count) = (ids.as_ptr(), vals.as_ptr(), N as c_int);
        let error = unsafe { uc_reg_read_batch(self.get_handle(), ids, vals, count) };
        (error == uc_error::OK).then_some(values).ok_or(error)
    }

    fn reg_write_batch<const N: usize, R: Into<i32>>(
        &mut self,
        pairs: [(R, u64); N],
    ) -> Result<(), uc_error> {
        let vals = pairs.each_ref().map(|(_, x)| (&raw const *x).cast());
        let ids: [i32; N] = std::array::from_fn(|index| unsafe {
            std::ptr::from_ref(&pairs[index].0).read().into()
        });
        let (ids, vals, count) = (ids.as_ptr(), vals.as_ptr(), N as c_int);
        let error = unsafe { uc_reg_write_batch(self.get_handle(), ids, vals, count) };
        std::mem::forget(pairs);
        (error == uc_error::OK).then_some(()).ok_or(error)
    }

    fn remove_cache(&self, addr: u64, end: u64) -> Result<(), uc_error> {
        let control = UC_CTL_IO_WRITE << 30 | ControlType::TbRemoveCache as u32;
        let error = unsafe { uc_ctl(self.get_handle(), control, addr, end) };
        (error == uc_error::OK).then_some(()).ok_or(error)
    }
}
