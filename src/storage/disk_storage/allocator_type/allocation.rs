pub trait AllocationType{
    fn get_ith_header_list(&self, index: u64) -> u64;
    fn set_ith_header_list(&self, index: u64, offset: u64);
    fn bump_allocate(&self, size: u64) -> u64;
}
