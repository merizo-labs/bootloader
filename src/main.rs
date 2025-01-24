#![no_std]
#![no_main]
use uefi::{helpers::init, prelude::*};

#[entry]
fn boot_entry() -> Status {
    init().unwrap();
    Status::SUCCESS
}
