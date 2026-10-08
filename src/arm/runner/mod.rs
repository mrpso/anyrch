use super::*;

#[cfg(feature = "unicorn")]
mod unicorn;

#[cfg(target_arch = "arm")]
mod native;

pub enum ArmRunner {
    #[cfg(target_arch = "arm")]
    Native(self::native::Native),
    #[cfg(feature = "unicorn")]
    Unicorn(self::unicorn::Unicorn),
}

impl ArmRunner {
    pub fn create() -> Result<Self> {
        cfg_select! {
            target_arch = "arm" => Ok(Self::Native(self::native::Native::create()?)),
            feature = "unicorn" => Ok(Self::Unicorn(self::unicorn::Unicorn::create()?)),
            _ => compile_error!("unicorn feature is required for arm architecture"),
        }
    }

    #[inline(always)]
    pub fn clear_cache(&self, addr: u32, end: u32) {
        match self {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.clear_cache(),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.clear_cache(addr as u64, end as u64),
        }
    }

    #[inline(always)]
    pub fn translate(&self, addr: u32) -> *mut u8 {
        match self {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.translate(addr),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.translate(addr as u64),
        }
    }

    #[inline(always)]
    pub fn memmap(&self, size: usize, perms: Perms) -> u32 {
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.memmap(size, perms),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.memmap(size as u64, perms) as u32,
        }
    }

    #[inline(always)]
    pub fn munmap(&self, addr: u32, size: usize) {
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.munmap(addr, size),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.munmap(addr as u64, size as u64),
        }
    }

    #[inline(always)]
    pub fn malloc(&self, size: usize) -> u32 {
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.malloc(size),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.malloc(size as u64) as u32,
        }
    }

    #[inline(always)]
    pub fn free(&self, addr: u32) {
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.free(addr),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.free(addr as u64),
        }
    }

    #[inline(always)]
    pub fn register<F: FnMut(u32) + 'static>(&self, mut f: F) -> u32 {
        // let runner = std::ptr::from_ref(self);
        // let f = move |sp: u64| f(unsafe { &*runner }, sp as u32);
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.register(f),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.register(move |sp| f(sp as u32)) as u32,
        }
    }

    #[inline(always)]
    pub fn call<F: FnOnce(u32) -> u32>(&self, addr: u32, f: F) -> Result<[u8; 8]> {
        match self.as_mut() {
            #[cfg(target_arch = "arm")]
            Self::Native(native) => native.call(addr, f),
            #[cfg(feature = "unicorn")]
            Self::Unicorn(unicorn) => unicorn.call(addr as u64, |sp| f(sp as u32) as u64),
        }
    }
}

impl ArmRunner {
    #[inline(always)]
    fn as_mut(&self) -> &mut Self {
        unsafe { std::ptr::from_ref(self).cast_mut().as_mut_unchecked() }
    }
}
