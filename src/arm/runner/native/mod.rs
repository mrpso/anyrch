use hashbrown::HashMap;

use super::{ArmRunner, Result, Runner};

pub struct Native {
    mmap: HashMap<*mut u8, usize>,
    used: HashMap<*mut u8, usize>,
}

impl Native {
    pub fn create() -> Result<Self> {
        // Ok(Self {})
        todo!()
    }
}

impl ArmRunner for Native {}

impl Runner for Native {
    fn translate(&self, addr: u64) -> *mut u8 {
        std::ptr::with_exposed_provenance_mut(addr as usize)
    }

    fn memmap(&mut self, size: u64, perm: i32) -> u64 {
        let addr = std::ptr::null_mut();
        let len = size as usize;
        let prot = perm & (libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC);
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
        // 如果mmap失败，返回 -1 值为 libc::MAP_FAILED
        let ptr = unsafe { libc::mmap(addr, len, prot, flags, -1, 0) };
        if ptr != libc::MAP_FAILED {
            self.mmap.insert(ptr.cast(), len);
            ptr as usize as u64
        } else {
            0
        }
    }

    fn malloc(&mut self, size: u64) -> u64 {
        let size = size as usize;
        let ptr = unsafe { libc::malloc(size) };
        if ptr.addr() != 0 {
            self.used.insert(ptr.cast(), size);
            ptr.addr() as u64
        } else {
            // let errno = unsafe { *libc::__errno() };
            0
        }
    }

    fn free(&mut self, addr: u64) {
        let ptr = self.translate(addr);
        if let Some(_) = self.used.remove(&ptr) {
            unsafe { libc::free(ptr.cast()) };
        } else if let Some(len) = self.mmap.remove(&ptr.cast()) {
            // ### 失败时的常见错误码 (`errno`)
            //
            // 当 `munmap` 返回 `-1` 时，你可以通过查看系统全局变量 `errno`
            // 在 Rust 中可以通过 `std::io::Error::last_os_error()` 获取 来定位失败原因
            //
            // * **`EINVAL` (Invalid Argument)**：最常见原因。
            // * 传入的地址 `addr` **没有按照系统的页大小（Page Size，通常是 4KB）对齐**。
            // * 传入的长度 `len` 为 0。
            // * 取消映射的虚拟地址区间超出了进程允许的地址空间范围。
            //
            // * **`ENOMEM`**：欲取消映射的地址区间中，包含了非法的、未映射的或超出系统上限的内存页。
            let _ret = unsafe { libc::munmap(ptr.cast(), len) };
        }
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        for mmap in self.mmap.values() {
            unsafe { libc::munmap(mmap as _, mmap) };
        }

        for mmap in self.used.values() {
            unsafe { libc::free(mmap as _, mmap) };
        }
    }
}
