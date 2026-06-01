use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStringExt;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, IMAGE_NT_HEADERS64, IMAGE_SCN_MEM_EXECUTE, IMAGE_SECTION_CHARACTERISTICS, IMAGE_SECTION_HEADER};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW, MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPPROCESS};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

pub fn wait_for_handle(target: &str) -> (HANDLE, *const c_void) {
    loop {
        unsafe {
            let snap = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) { Ok(h) => h, Err(_) => { std::thread::sleep(Duration::from_millis(100)); continue; } };
            let mut pe = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if !Process32FirstW(snap, &mut pe).as_bool() {
                CloseHandle(snap).unwrap();
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            let found = loop {
                let len = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                let name = OsString::from_wide(&pe.szExeFile[..len]).to_string_lossy().to_string();
                if name.eq_ignore_ascii_case(target) { break Some(pe.th32ProcessID); }
                if !Process32NextW(snap, &mut pe).as_bool() { break None; }
            };
            CloseHandle(snap).unwrap();

            let pid = match found { Some(p) => p, None => { std::thread::sleep(Duration::from_millis(100)); continue; } };
            let handle = match OpenProcess(PROCESS_ALL_ACCESS, false, pid) { Ok(h) if !h.is_invalid() => h, _ => { std::thread::sleep(Duration::from_millis(100)); continue; } };

            // grab module base from the same process while we have the pid
            let base = match CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid) {
                Err(_) => { CloseHandle(handle).unwrap(); std::thread::sleep(Duration::from_millis(100)); continue; }
                Ok(msnap) => {
                    let mut me = MODULEENTRY32W { dwSize: size_of::<MODULEENTRY32W>() as u32, ..Default::default() };
                    let mut found_base = std::ptr::null();
                    if Module32FirstW(msnap, &mut me).as_bool() {
                        loop {
                            let len = me.szModule.iter().position(|&c| c == 0).unwrap_or(me.szModule.len());
                            let mname = OsString::from_wide(&me.szModule[..len]).to_string_lossy().to_string();
                            if mname.eq_ignore_ascii_case(target) { found_base = me.modBaseAddr as *const c_void; break; }
                            if !Module32NextW(msnap, &mut me).as_bool() { break; }
                        }
                    }
                    CloseHandle(msnap).unwrap();
                    found_base
                }
            };

            if base.is_null() { CloseHandle(handle).unwrap(); std::thread::sleep(Duration::from_millis(100)); continue; }
            eprintln!("{} module base: {:p}", target, base);
            return (handle, base);
        }
    }
}

const ANY_: i16 = -1;
pub unsafe fn find_pattern(address: *const u8, limit: usize, pattern: *const i16, length: usize) -> *const u8 {
    let mut current = address;
    let end = address.add(limit);
    let mut pattern_pos: usize = 0;

    while pattern_pos < length && current < end {
        let p = *pattern.add(pattern_pos);
        let c = *current as i16;

        if p == ANY_ || p == c {
            pattern_pos += 1;
            current = current.add(1);
        } else if pattern_pos != 0 { pattern_pos = 0; } else { current = current.add(1); }
    }
    if pattern_pos == length { current.sub(length) } else { std::ptr::null() }
}

pub unsafe fn find_pattern_ex(process: HANDLE, address: *const u8, limit: usize, pattern: *const i16, length: usize) -> *const u8 {
    const BUF_SIZE: usize = 0x10000;
    let mut current = address;
    let mut left = limit;
    let mut buf = [0u8; BUF_SIZE];

    while left > length {
        let count = if left >= BUF_SIZE { BUF_SIZE } else { left };
        left -= count - length;
        let mut bytes_read: usize = 0;
        let read_ok = ReadProcessMemory(process, current as *const c_void, buf.as_mut_ptr() as *mut c_void, count, &mut bytes_read, ).as_bool();
        if !read_ok || bytes_read == 0 {
            current = current.add(count - length);
            continue;
        }

        let pattern_pos = find_pattern(buf.as_ptr(), bytes_read, pattern, length);
        if !pattern_pos.is_null() { return current.add(pattern_pos.offset_from(buf.as_ptr()) as usize); }
        current = current.add(count - length);
    }
    std::ptr::null()
}

unsafe fn try_read_memory(process: HANDLE, address: *const c_void, buffer: &mut [u8]) -> bool {
    let mut bytes_read = 0;
    ReadProcessMemory(process, address, buffer.as_mut_ptr() as *mut c_void, buffer.len(), &mut bytes_read).as_bool() && bytes_read == buffer.len()
}

unsafe fn find_pattern_ex_in_module(process: HANDLE, module: *const c_void, filter: IMAGE_SECTION_CHARACTERISTICS, pattern: *const i16, length: usize) -> *const u8 {
    let mut header = [0u8; 0x1000];
    if !try_read_memory(process, module, &mut header) { return std::ptr::null(); }

    let dos_header = &*(header.as_ptr() as *const IMAGE_DOS_HEADER);
    if dos_header.e_magic != 0x5A4D { return std::ptr::null(); }

    let nt_headers = &*((header.as_ptr().add(dos_header.e_lfanew as usize)) as *const IMAGE_NT_HEADERS64);
    let section_headers = (nt_headers as *const IMAGE_NT_HEADERS64).add(1) as *const IMAGE_SECTION_HEADER;
    let section_count = nt_headers.FileHeader.NumberOfSections as usize;

    for i in 0..section_count {
        let section = &*section_headers.add(i);
        if (section.Characteristics & filter) != filter { continue; }
        let section_addr = (module as *const u8).add(section.VirtualAddress as usize);
        let scan_size = section.Misc.VirtualSize.max(section.SizeOfRawData) as usize;
        let pos = find_pattern_ex(process, section_addr, scan_size, pattern, length);
        if !pos.is_null() { return pos; }
    }
    std::ptr::null()
}

// Works under Wine/Proton; Windows support is WIP
pub unsafe fn find_fps_var(process: HANDLE, module_base: *const c_void) -> *mut u32 {
    let executable = module_base;

    let setter_call_pattern: [i16; 11] = [
        0xB9, 0x3C, 0x00, 0x00, 0x00, // B9 3C000000 mov ecx, 60
        0xE8, ANY_, ANY_, ANY_, ANY_, // E8 ???????? call setter
        0x80,                         // 80 ? cmp byte [x], y
    ];

    let setter_call = find_pattern_ex_in_module(process, executable, IMAGE_SCN_MEM_EXECUTE, setter_call_pattern.as_ptr(), setter_call_pattern.len());
    if setter_call.is_null() {
        eprintln!("Could not find setter call pattern");
        return std::ptr::null_mut();
    }

    let mut bytes_at_addr = [0u8; 6];
    let mut potential_mov = setter_call.add(5);

    for _ in 0..32 {
        let mut bytes_read: usize = 0;
        let read_ok = ReadProcessMemory(process, potential_mov as *const c_void, bytes_at_addr.as_mut_ptr() as *mut c_void, bytes_at_addr.len(), &mut bytes_read).as_bool();
        if !read_ok || bytes_read != bytes_at_addr.len() { return std::ptr::null_mut(); }

        if bytes_at_addr[0] == 0xE9 || bytes_at_addr[0] == 0xE8 {
            let rel = i32::from_le_bytes(bytes_at_addr[1..5].try_into().unwrap());
            potential_mov = potential_mov.add((rel as isize + 5) as usize);
        } else { break; }
    }

    if bytes_at_addr[0] != 0x89 || bytes_at_addr[1] != 0x0D {
        eprintln!("Could not find 'mov [fps], ecx' (got {:02X} {:02X})", bytes_at_addr[0], bytes_at_addr[1]);
        return std::ptr::null_mut();
    }
    let fps_offset = i32::from_le_bytes(bytes_at_addr[2..6].try_into().unwrap());
    (potential_mov.add(6).offset(fps_offset as isize)) as *mut u32
}

#[repr(C, packed(2))]
#[allow(non_camel_case_types)]
pub struct IMAGE_DOS_HEADER {
    pub e_magic: u16,
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_csum: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
    pub e_ovno: u16,
    pub e_res: [u16; 4],
    pub e_oemid: u16,
    pub e_oeminfo: u16,
    pub e_res2: [u16; 10],
    pub e_lfanew: i32,
}