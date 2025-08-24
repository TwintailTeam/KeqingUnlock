use std::ffi::{OsString};
use std::os::windows::ffi::OsStringExt;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW, MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPPROCESS};
use windows::Win32::System::Threading::{GetExitCodeProcess, GetProcessId, OpenProcess, PROCESS_ALL_ACCESS};

pub fn wait_for_handle_by_name(target: &str) -> HANDLE {
    loop {
        unsafe {
            let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(h) => h,
                Err(_) => { std::thread::sleep(Duration::from_millis(100)); continue; }
            };

            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if !Process32FirstW(snapshot, &mut entry).as_bool() {
                CloseHandle(snapshot).unwrap();
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            let handle = loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let exe_name = OsString::from_wide(&entry.szExeFile[..len]).to_string_lossy().to_string();
                if exe_name.eq_ignore_ascii_case(target) {
                    let proc_handle = OpenProcess(PROCESS_ALL_ACCESS, false, entry.th32ProcessID);
                    break proc_handle;
                }
                if !Process32NextW(snapshot, &mut entry).as_bool() { break Ok(HANDLE::default()); }
            };

            CloseHandle(snapshot).unwrap();
            let handle = handle.unwrap();
            if !handle.is_invalid() { return handle; }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

pub fn is_alive(handle: HANDLE) -> bool {
    unsafe {
        let mut exitcode: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut exitcode as *mut _).as_bool();
        if !ok { return false; }
        exitcode == STILL_ACTIVE.0 as u32
    }
}

pub fn get_pid_from_handle(process_handle: HANDLE) -> u32 { unsafe { GetProcessId(process_handle) } }

pub fn get_module_base(pid: u32, module_name: &str) -> Option<(usize, usize)> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid).unwrap();
        if snapshot == INVALID_HANDLE_VALUE { return None; }

        let mut module_entry = MODULEENTRY32W::default();
        module_entry.dwSize = size_of::<MODULEENTRY32W>() as u32;

        if Module32FirstW(snapshot, &mut module_entry).as_bool() {
            loop {
                let mod_name = {
                    let len = module_entry.szModule.iter().position(|&c| c == 0).unwrap_or(module_entry.szModule.len());
                    OsString::from_wide(&module_entry.szModule[..len]).to_string_lossy().into_owned()
                };
                if mod_name.eq_ignore_ascii_case(module_name) {
                    CloseHandle(snapshot);
                    return Some((module_entry.modBaseAddr as usize, module_entry.modBaseSize as usize));
                }
                if !Module32NextW(snapshot, &mut module_entry).as_bool() { break; }
            }
        }
        CloseHandle(snapshot);
        None
    }
}

fn pattern_scan(data: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    for i in 0..=data.len() - pattern.len() {
        if pattern.iter().enumerate().all(|(j, x)| {
            if let Some(b) = x { &data[i + j] == b } else { true }
        }) { return Some(i); }
    }
    None
}

pub fn read_process_memory_safe(handle: HANDLE, base: usize, size: usize, ) -> windows::core::Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(size);
    let mut offset = 0;
    let chunk_size = 0x1000; // 4KB chunks

    while offset < size {
        let read_size = std::cmp::min(chunk_size, size - offset);
        let mut chunk = vec![0u8; read_size];
        let mut bytes_read = 0;
        let success = unsafe { ReadProcessMemory(handle, (base + offset) as *const _, chunk.as_mut_ptr() as *mut _, read_size, &mut bytes_read).as_bool() };
        if success && bytes_read > 0 { buffer.extend_from_slice(&chunk[..bytes_read]); offset += bytes_read; } else { offset += read_size; }
    }
    Ok(buffer)
}

pub fn read_i32(handle: HANDLE, address: usize) -> windows::core::Result<i32> {
    let mut buffer = 0i32;
    let mut bytes_read = 0;
    let success = unsafe { ReadProcessMemory(handle, address as *const _, &mut buffer as *mut _ as *mut _, size_of::<i32>(), &mut bytes_read).as_bool() };
    if success && bytes_read == size_of::<i32>() { Ok(buffer) } else { Err(windows::core::Error::from_win32()) }
}

pub fn write_i32(handle: HANDLE, address: usize, value: i32) -> windows::core::Result<()> {
    let mut bytes_written = 0;
    let success = unsafe { WriteProcessMemory(handle, address as *mut _, &value as *const _ as *const _, size_of::<i32>(), &mut bytes_written).as_bool() };
    if success && bytes_written == size_of::<i32>() { Ok(()) } else { Err(windows::core::Error::from_win32()) }
}

// "B9 3C 00 00 00 E8"
pub fn get_fps_address(buffer: &[u8], base_addr: usize) -> Option<usize> {
    let pattern = vec![Some(0xB9), Some(0x3C), Some(0x00), Some(0x00), Some(0x00), Some(0xE8), ];
    let offset = pattern_scan(buffer, &pattern)?;
    let rip = offset + 5;
    if rip + 5 > buffer.len() { return None; }
    let disp_bytes = &buffer[rip + 1..rip + 5];
    let disp = i32::from_le_bytes(disp_bytes.try_into().ok()?) as isize;
    let call_next_instr = base_addr + rip + 5;
    Some((call_next_instr as isize + disp) as usize)
}