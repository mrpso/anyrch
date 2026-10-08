pub trait Runner {
    fn translate(&self, addr: u64) -> *mut u8;

    fn memmap(&mut self, size: u64, perms: i32) -> u64;

    fn malloc(&mut self, size: u64) -> u64;

    fn free(&mut self, addr: u64);

    fn register(&mut self, func: Box<dyn Fn()>) -> u64;
}
