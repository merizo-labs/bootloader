#![no_std]
#![no_main]

//TODO: Add error handling.
extern crate alloc;
use alloc::{boxed::Box, vec::Vec};
use uefi::{fs::FileSystem, prelude::*, CStr16, Result};
use xmas_elf::ElfFile;
const KERNEL_PATH: &str = concat!("mkernel-", env!("CARGO_PKG_VERSION"), ".elf");

#[entry]
fn boot_entry() -> Status {
    uefi::helpers::init().unwrap();
    let img = boot::image_handle();
    uefi::helpers::init().unwrap();
    let sfs = boot::get_image_file_system(img).unwrap();
    let mut fs = FileSystem::new(sfs);
    let kernel_file = locate_kernel(&mut fs).unwrap();
    let entry = load_kernel(kernel_file).unwrap();
    entry();
    Status::SUCCESS
}

fn load_kernel(elf: ElfFile<'static>) -> Result<extern "C" fn() -> !, &'static str> {
    for program_header in elf.program_iter() {
        if program_header
            .get_type()
            .map_err(|e| uefi::Error::new(Status::LOAD_ERROR, e))?
            == xmas_elf::program::Type::Load
        {
            pht_process::process_load(&elf, &program_header)?;
        }
    }
    let entry = elf.header.pt2.entry_point();
    let entry_fn: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry) };
    Ok(entry_fn)
}
/// Program header types process
mod pht_process {
    use uefi::{prelude::*, Result};
    pub fn process_load(
        elf: &xmas_elf::ElfFile<'static>,
        program_header: &xmas_elf::program::ProgramHeader,
    ) -> Result<(), &'static str> {
        let mem_size = program_header.mem_size() as usize;
        let phys_addr = program_header.physical_addr() as u64;
        let pages = (mem_size + 4095) / 4096;
        boot::allocate_pages(
            boot::AllocateType::Address(phys_addr),
            boot::MemoryType::LOADER_DATA,
            pages,
        )
        .map_err(|_| uefi::Error::new(Status::LOAD_ERROR, "Failed to allocate pages"))?;
        let data = program_header.get_data(&elf).map_err(|_| {
            uefi::Error::new(Status::LOAD_ERROR, "Failed to get program header data")
        })?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                as_ptr_segment_data(data),
                phys_addr as *mut u8,
                program_header.file_size() as usize,
            );
        }
        Ok(())
    }
    fn as_ptr_segment_data<'a>(data: xmas_elf::program::SegmentData<'a>) -> *const u8 {
        match data {
            xmas_elf::program::SegmentData::Undefined(data) => data.as_ptr(),
            xmas_elf::program::SegmentData::Dynamic32(data) => data.as_ptr() as *const u8,
            xmas_elf::program::SegmentData::Dynamic64(data) => data.as_ptr() as *const u8,
            xmas_elf::program::SegmentData::Note64(_name, desc) => desc.as_ptr(),
            xmas_elf::program::SegmentData::Empty => core::ptr::null(),
        }
    }
}
fn locate_kernel(fs: &mut FileSystem) -> Result<ElfFile<'static>, &'static str> {
    let codes = KERNEL_PATH.encode_utf16().collect::<Vec<u16>>();
    let kernel_path = CStr16::from_u16_with_nul(&codes).map_err(|_| {
        uefi::Error::new(
            Status::LOAD_ERROR,
            "Failed to convert kernel path into cstr16",
        )
    })?;

    if let Ok(bin) = fs.read(kernel_path) {
        let bin = Box::leak(Box::new(bin));
        Ok(ElfFile::new(bin)
            .map_err(|_| uefi::Error::new(Status::LOAD_ERROR, "Invalid kernel ELF file"))?)
    } else {
        Err(uefi::Error::new(Status::LOAD_ERROR, "Kernel not found"))
    }
}
