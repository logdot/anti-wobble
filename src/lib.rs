//! Simple mod that patches highfleet to prevent gui shacking

#![deny(missing_docs)]

use std::{arch::asm, ffi::c_void};

use patchy::Patch;
use windows::Win32::{Foundation::{CloseHandle, HMODULE}, System::{LibraryLoader::FreeLibraryAndExitThread, Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS}, SystemServices::DLL_PROCESS_ATTACH, Threading::{CreateThread, THREAD_CREATION_FLAGS}}};

mod patchy;

#[no_mangle]
#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "system" fn DllMain(dll_module: HMODULE, call_reason: u32, _: *mut ()) -> bool {
    if call_reason != DLL_PROCESS_ATTACH {
        return true;
    }

    let handle = CreateThread(
        None,
        0,
        Some(attach),
        Some(std::ptr::addr_of!(dll_module).cast()),
        THREAD_CREATION_FLAGS(0),
        None,
    )
    .unwrap();
    CloseHandle(handle);

    true
}

unsafe extern "system" fn attach(handle: *mut c_void) -> u32 {
    let mut address;
    if cfg!(feature = "1_151") {
        address = 0x1400240c0;
    } else if cfg!(feature = "1_163") {
        address = 0x1400256e0;
    } else {
        // Default to 1.163
        address = 0x1400256e0;
    }

    let mut old_protect = PAGE_PROTECTION_FLAGS(0);

    VirtualProtect(
        address as *mut c_void,
        0x100,
        PAGE_EXECUTE_READWRITE,
        &mut old_protect as *mut _,
    ).unwrap();

    let p = Patch::patch_call(address, set_dumpable as *const (), 6);
    std::mem::forget(p);

    VirtualProtect(
        address as *mut c_void,
        0x100,
        old_protect,
        &mut old_protect as *mut _,
    ).unwrap();

    FreeLibraryAndExitThread(HMODULE(handle as _), 0);
}

#[no_mangle]
#[cfg(feature = "1_151")]
unsafe extern fn set_dumpable() {
    asm! {
        "mov byte ptr [rsi + 0x8e6], 0",
        out("rsi") _,
    }
}


#[no_mangle]
#[cfg(any(feature = "1_163", not(any(feature = "1_151", feature = "1_163"))))]
unsafe extern fn set_dumpable() {
    asm! {
        "mov byte ptr [rsi + 0x91E], 0",
        out("rsi") _,
    }
}
