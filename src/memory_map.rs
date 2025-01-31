use uefi::{
    boot::memory_map,
    mem::memory_map::{MemoryMapOwned, MemoryType},
    Result,
};

fn mem_map() -> Result<MemoryMapOwned> {
    let mtype = MemoryType::MAX;
    let map = memory_map(mtype).unwrap();

    Ok(map)
}
