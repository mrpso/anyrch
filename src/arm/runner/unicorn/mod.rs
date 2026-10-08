use std::{alloc::Layout, collections::BTreeMap, ops::Range};

use hashbrown::{HashMap, HashSet};
use unicorn_engine::{
    RegisterARM::*,
    Unicorn as UnicornEngine,
    unicorn_const::{Arch, Mode, Permission},
};

use crate::unicorn::UnicornExtend;

use super::{Perms, Result};

const PAGE_SIZE: u64 = 0x1000;
const ADDR_START: u64 = 0x00010000;
const ADDR_END: u64 = 0xF0000000;

// const STOP_ADDR: u64 = 0xF0C105E0;
const STACK_ADDR: u64 = 0xFFFF0000;
const STACK_SIZE: usize = 0x1000 * 4; // 16KB
const STACK_TOP: u64 = STACK_ADDR + STACK_SIZE as u64;

const CALL_ADDR: u64 = 0xFF000000;
const CALL_END: u64 = CALL_ADDR + 8;
const CALL_CODE: &[u8] = &[
    0x0F, 0x00, 0xBD, 0xE8, // pop {r0, r1, r2, r3}
    0x3C, 0xFF, 0x2F, 0xE1, // blx r12
    // 多指令填充，确保跳过停止地址
    0x00, 0x00, 0xA0, 0xE1, // mov r0, r0
    0x00, 0x00, 0xA0, 0xE1, // mov r0, r0
];

const FUNC_CODE: &[u8] = &[
    // 0x0F, 0x00, 0x2D, 0xE9, // push {r0, r1, r2, r3}
    // 0x0F, 0x00, 0xBD, 0xE8, // pop  {r0, r1, r2, r3}
    // 0x1E, 0xFF, 0x2F, 0xE1, // bx lr
    0x0F, 0xB4, // push {r0, r1, r2, r3}
    0x0F, 0xBC, // pop  {r0, r1, r2, r3}
    0x70, 0x47, // bx lr
];

pub struct Unicorn {
    engine: UnicornEngine<'static, ()>,
    mmap: BTreeMap<u64, Block>,
    // 维护已映射但尚未使用的RX权限指令虚拟地址块
    code: Vec<Range<u64>>,
    // 维护已映射但尚未使用的RW权限空闲虚拟地址块
    free: Vec<Range<u64>>,
    // 跟踪每个已分配给用户的地址块大小，方便 free 时精准回收: Virtual Start -> Size
    used: HashMap<u64, u64>,
    // 执行前需要清除的指令缓存的虚拟地址块
    clear: HashSet<Range<u64>>,
}

struct Block {
    layout: Layout,
    ptr: *mut u8,
    end: u64,
}

#[derive(Clone, Copy)]
enum Kind {
    RX,
    RW,
}

impl Unicorn {
    pub fn create() -> Result<Self> {
        let mut unicorn = UnicornEngine::new(Arch::ARM, Mode::ARM)?;
        let mut mmap = BTreeMap::new();
        let mut code = Vec::with_capacity(3);

        let layout = Layout::from_size_align(STACK_SIZE, PAGE_SIZE as usize).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        let perms = Permission::READ | Permission::WRITE;
        unsafe { unicorn.mem_map_ptr(STACK_ADDR, STACK_SIZE, perms, ptr.cast()) }?;
        mmap.insert(STACK_ADDR, Block::new(layout, ptr, STACK_TOP));
        unicorn.reg_write(SP, STACK_TOP)?;

        let size = AlignUp!(CALL_CODE.len() as u64, PAGE_SIZE);
        let layout = Layout::from_size_align(size as usize, PAGE_SIZE as usize).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        let perms = Permission::READ | Permission::EXEC;
        unsafe { ptr.copy_from(CALL_CODE.as_ptr(), CALL_CODE.len()) };
        unsafe { unicorn.mem_map_ptr(CALL_ADDR, layout.size(), perms, ptr.cast()) }?;
        mmap.insert(CALL_ADDR, Block::new(layout, ptr, CALL_ADDR + size));
        code.push(CALL_ADDR + CALL_CODE.len() as u64..CALL_ADDR + size);

        #[allow(unused)]
        {
            let mut max = 0;
            unicorn.add_code_hook(0, 0xFFFFFFFF, move |uc, addr, len| {
                // let sp = uc.reg_read(SP).unwrap();
                // if STACK_TOP - sp > max {
                //     max = STACK_TOP - sp;
                //     println!("Max Use Stack: 0x{:04x}", max);
                // }
                // let mut inst = [0u8; 4];
                // uc.mem_read(addr, &mut inst[..len as usize]).unwrap();
                // println!(
                //     "Hook Addr: 0x{:08x} Len: {}, Inst: {:02x}{:02x}{:02x}{:02x}",
                //     addr, len, inst[0], inst[1], inst[2], inst[3]
                // );
            })?;
        }

        Ok(Self {
            engine: unicorn,
            mmap,
            code,
            free: Vec::with_capacity(10),
            used: HashMap::with_capacity(100),
            clear: HashSet::with_capacity(3),
        })
    }

    #[inline(always)]
    pub fn clear_cache(&self, addr: u64, end: u64) {
        self.engine.remove_cache(addr, end).unwrap();
    }

    pub fn translate(&self, addr: u64) -> *mut u8 {
        match self.mmap.range(..=addr).next_back() {
            Some((start, block)) if addr < block.end => {
                let offset = addr - *start;
                block.ptr.wrapping_add(offset as _)
            }
            _ => std::ptr::null_mut(),
        }
    }

    #[inline(always)]
    pub fn memmap(&mut self, size: u64, perms: Perms) -> u64 {
        self.mmap(size, unsafe { std::mem::transmute(perms) })
    }

    #[inline(always)]
    pub fn munmap(&mut self, addr: u64, _size: u64) {
        self.mmap.remove(&addr);
    }

    #[inline(always)]
    pub fn malloc(&mut self, size: u64) -> u64 {
        let size = AlignUp!(size, 8);
        let addr = self.alloc(Kind::RW, size);
        // 3. 记录已分配块的大小并返回地址
        self.used.insert(addr, size);
        addr
    }

    pub fn free(&mut self, addr: u64) {
        // 1. 查找此地址是否有分配记录
        if let Some(size) = self.used.remove(&addr) {
            // 2. 将释放的内存归还到 free_list
            self.free.push(addr..addr + size as u64);
            self.free.sort_by_key(|r| r.start);
            self.free.dedup_by(|next, cur| {
                let merge = cur.end == next.start;
                merge.then(|| cur.end = next.end);
                merge
            });
        };
    }

    pub fn call<F: FnOnce(u64) -> u64>(&mut self, addr: u64, f: F) -> Result<[u8; 8]> {
        let uc = &mut self.engine;
        if !self.clear.is_empty() {
            for range in std::mem::replace(&mut self.clear, HashSet::new()) {
                uc.remove_cache(range.start, range.end).unwrap();
            }
        }

        let sp = uc.reg_read(SP)?;
        uc.reg_write_batch([(SP, f(sp - 16)), (R12, addr)])?;
        if let Err(e) = uc.emu_start(CALL_ADDR, CALL_END, 0, 0) {
            let mut buf = [0u8; 8];
            let [pc, lr] = uc.reg_read_batch([PC, LR])?;
            println!("执行错误地址 0x{:08x} 返回地址：0x{:08x}", pc, lr);
            uc.mem_read(pc.checked_sub(8).unwrap_or_else(|| lr - 8), &mut buf)?;
            let bytes = bytes::Bytes::copy_from_slice(&buf);
            println!("最后指令：{:x}", bytes);
            return Err(e.into());
        };

        uc.reg_write(SP, sp)?;
        let regs = uc.reg_read_batch([R0, R1])?;
        Ok(unsafe { std::mem::transmute(regs.map(|x| (x as u32).to_le())) })
    }
}

impl Unicorn {
    pub fn register<F: FnMut(u64) + 'static>(&mut self, mut f: F) -> u64 {
        let size = AlignUp!(FUNC_CODE.len() as u64, 4);
        let addr = self.alloc(Kind::RX, size);
        let ptr = self.translate(addr);
        unsafe { ptr.copy_from(FUNC_CODE.as_ptr(), FUNC_CODE.len()) };

        let (begin, end) = (addr + 2, addr + 3);
        let _ = self.engine.add_code_hook(begin, end, move |uc, _, _| {
            let lr = uc.reg_read(LR).unwrap();
            f(uc.reg_read(SP).unwrap());
            uc.reg_write(LR, lr).unwrap();
        });

        let start = AlignDown!(addr, PAGE_SIZE);
        let end = AlignUp!(addr + size, PAGE_SIZE);
        self.clear.insert(start..end);

        // Thumb 模式
        addr + 1
    }
}

impl Unicorn {
    fn mmap(&mut self, size: u64, perms: Permission) -> u64 {
        let size = AlignUp!(size, PAGE_SIZE);
        let range = ADDR_START..=ADDR_END - size;
        let mut count = 0;
        let (start, end) = loop {
            let addr = AlignDown!(rand::random_range(range.clone()), PAGE_SIZE);
            let end = addr + size;

            // 只做一次 BTreeMap 查找：拿到 key < end 的最后一个 Block
            if let Some((start, block)) = self.mmap.range(..end).next_back() {
                // 情况 1: 该 Block 的起始地址在 [addr, end) 之间 -> 冲突
                // 情况 2: 该 Block 的起始地址 < addr，但 end > addr -> 冲突
                if *start >= addr || block.end > addr {
                    if count >= 100 {
                        panic!("Unicorn随机分配地址次数大于100次，分配长度：0x{:x}", size)
                    }
                    count += 1;
                    continue;
                }
            }

            break (addr, end);
        };

        let size = size as usize;
        let layout = Layout::from_size_align(size, PAGE_SIZE as usize).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        // let perms = Permission::ALL;
        // let perms = unsafe { std::mem::transmute(perm) };
        unsafe { self.engine.mem_map_ptr(start, size, perms, ptr.cast()) }.unwrap();
        self.mmap.insert(start, Block { layout, ptr, end });

        start
    }

    fn alloc(&mut self, kind: Kind, size: u64) -> u64 {
        let ranges = match kind {
            Kind::RX => &mut self.code,
            Kind::RW => &mut self.free,
        };

        let mut iter = ranges.iter_mut().enumerate();
        if let Some((index, range)) = iter.find(|(_, r)| (r.end - r.start) >= size) {
            return match range.end - range.start == size {
                true => ranges.remove(index).start,
                false => (range.start, range.start += size).0,
            };
        }

        let map_size = AlignUp!(size, PAGE_SIZE);
        let addr = self.mmap(map_size, kind.perms());
        if map_size > size {
            // 将多余映射的页面存入 free_list
            // free.push(addr + size..addr + map_size);
            let range = addr + size..addr + map_size;
            match kind {
                Kind::RX => self.code.push(range),
                Kind::RW => self.free.push(range),
            }
        }

        addr

        // match iter.find(|(_, r)| (r.end - r.start) >= size) {
        //     Some((i, r)) => match r.end - r.start == size {
        //         true => free.remove(i).start,
        //         false => (r.start, r.start += size).0,
        //     },
        //     None => {
        //         let map_size = AlignUp!(size, PAGE_SIZE);
        //         let addr = self.mmap(map_size, kind.perms());
        //         if map_size > size {
        //             // 将多余映射的页面存入 free_list
        //             free.push(addr + size..addr + map_size);
        //         }

        //         addr
        //     }
        // }
    }
}

impl Block {
    fn new(layout: Layout, ptr: *mut u8, end: u64) -> Self {
        Self { layout, ptr, end }
    }
}

impl Kind {
    fn perms(&self) -> Permission {
        match self {
            Self::RX => Permission::READ | Permission::EXEC,
            Self::RW => Permission::READ | Permission::WRITE,
        }
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}
