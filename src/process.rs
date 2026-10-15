use std::ffi::c_void;
use std::io;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Memory::{
    VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessAffinityMask, GetProcessTimes, OpenProcess, SetPriorityClass, SetProcessAffinityMask, WaitForSingleObject,
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_VM_OPERATION,
    PROCESS_VM_READ, PROCESS_VM_WRITE, QueryFullProcessImageNameW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

const SYNCHRONIZE: u32 = 0x0010_0000;
const SCAN_CHUNK: usize = 1 << 20;

#[derive(Clone, Copy, PartialEq)]
pub enum Access {
    /// Affinity and priority only; this handle can neither read nor write the target's memory.
    SchedulingOnly,
    /// Reads memory but cannot change anything in the target.
    MemoryRead,
    /// Affinity and priority, plus memory read and write.
    Memory,
}

/// Restricts this process to `mask`; the processes it creates afterwards inherit this mask.
pub fn set_current_affinity(mask: usize) -> io::Result<()> {
    check(unsafe { SetProcessAffinityMask(GetCurrentProcess(), mask) })
}

/// Every logical processor of the system, as a mask.
pub fn system_affinity() -> io::Result<usize> {
    let (mut process, mut system) = (0usize, 0usize);
    check(unsafe { GetProcessAffinityMask(GetCurrentProcess(), &mut process, &mut system) })?;
    Ok(system)
}

fn filetime(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// The current time, in the same unit as `Process::creation_time`.
pub fn now() -> u64 {
    let mut time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    unsafe { GetSystemTimeAsFileTime(&mut time) };
    filetime(time)
}

/// True if the foreground window belongs to process `pid`.
pub fn is_foreground(pid: u32) -> bool {
    let mut owner = 0u32;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut owner) };
    owner == pid
}

/// True while the key with virtual-key code `vk` is down.
pub fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Finds the first running process whose image name matches one of `names` (case-insensitive).
pub fn find(names: &[String]) -> Option<(u32, String)> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut found = None;
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) };
    while more != 0 {
        let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
        if names.iter().any(|n| n.eq_ignore_ascii_case(&exe)) {
            found = Some((entry.th32ProcessID, exe));
            break;
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) };
    found
}

pub struct Process {
    handle: HANDLE,
    pub pid: u32,
    pub name: String,
}

impl Process {
    pub fn open(pid: u32, name: String, access: Access) -> io::Result<Self> {
        let scheduling = PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE;
        let read = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | SYNCHRONIZE;
        let rights = match access {
            Access::SchedulingOnly => scheduling,
            Access::MemoryRead => read,
            Access::Memory => scheduling | read | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
        };
        let handle = unsafe { OpenProcess(rights, 0, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { handle, pid, name })
    }

    pub fn is_running(&self) -> bool {
        unsafe { WaitForSingleObject(self.handle, 0) == WAIT_TIMEOUT }
    }

    pub fn affinity(&self) -> io::Result<usize> {
        let (mut process, mut system) = (0usize, 0usize);
        check(unsafe { GetProcessAffinityMask(self.handle, &mut process, &mut system) })?;
        Ok(process)
    }

    pub fn set_affinity(&self, mask: usize) -> io::Result<()> {
        check(unsafe { SetProcessAffinityMask(self.handle, mask) })
    }

    pub fn set_priority_class(&self, class: u32) -> io::Result<()> {
        check(unsafe { SetPriorityClass(self.handle, class) })
    }

    /// When the process was created, in hundreds of nanoseconds since 1601 (the Windows clock).
    /// Together with the pid it identifies a process unambiguously: a pid gets reused, not this pair.
    pub fn creation_time(&self) -> Option<u64> {
        let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        let ok = unsafe { GetProcessTimes(self.handle, &mut created, &mut exited, &mut kernel, &mut user) };
        (ok != 0).then(|| filetime(created))
    }

    /// Full path of the process executable.
    pub fn image_path(&self) -> Option<String> {
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(self.handle, 0, buffer.as_mut_ptr(), &mut len) };
        (ok != 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]))
    }

    pub fn read_bytes(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
        let mut done = 0usize;
        check(unsafe { ReadProcessMemory(self.handle, addr as *const c_void, buf.as_mut_ptr().cast(), buf.len(), &mut done) })?;
        if done != buf.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "partial read"));
        }
        Ok(())
    }

    pub fn write_bytes(&self, addr: usize, data: &[u8]) -> io::Result<()> {
        let mut done = 0usize;
        check(unsafe { WriteProcessMemory(self.handle, addr as *const c_void, data.as_ptr().cast(), data.len(), &mut done) })?;
        if done != data.len() {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "partial write"));
        }
        Ok(())
    }

    /// Reads `len` bytes in blocks; unreadable blocks stay zero instead of failing the whole read.
    pub fn read_lossy(&self, addr: usize, len: usize) -> Vec<u8> {
        let mut data = vec![0u8; len];
        for (index, block) in data.chunks_mut(0x1_0000).enumerate() {
            let _ = self.read_bytes(addr + index * 0x1_0000, block);
        }
        data
    }

    /// Zero-terminated string, if it is readable and only contains printable ASCII.
    pub fn read_c_string(&self, addr: usize, max: usize) -> Option<String> {
        let mut raw = vec![0u8; max];
        self.read_bytes(addr, &mut raw).ok()?;
        let len = raw.iter().position(|&b| b == 0)?;
        let text = &raw[..len];
        (!text.is_empty() && text.iter().all(|b| b.is_ascii_graphic() || *b == b' ')).then(|| String::from_utf8_lossy(text).into_owned())
    }

    pub fn read_u8(&self, addr: usize) -> io::Result<u8> {
        let mut b = [0u8; 1];
        self.read_bytes(addr, &mut b)?;
        Ok(b[0])
    }

    pub fn read_i32(&self, addr: usize) -> io::Result<i32> {
        let mut b = [0u8; 4];
        self.read_bytes(addr, &mut b)?;
        Ok(i32::from_le_bytes(b))
    }

    pub fn read_f32(&self, addr: usize) -> io::Result<f32> {
        let mut b = [0u8; 4];
        self.read_bytes(addr, &mut b)?;
        Ok(f32::from_le_bytes(b))
    }

    pub fn read_usize(&self, addr: usize) -> io::Result<usize> {
        let mut b = [0u8; 8];
        self.read_bytes(addr, &mut b)?;
        Ok(usize::from_le_bytes(b))
    }

    /// Walks the allocated, readable memory in `[start, end)` and returns every address where `pattern` appears.
    pub fn find_pattern(&self, start: usize, end: usize, pattern: &[u8], executable_only: bool) -> Vec<usize> {
        let mut hits = Vec::new();
        let mut addr = start;
        let mut chunk = vec![0u8; SCAN_CHUNK + pattern.len()];
        while addr < end {
            let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
            let got = unsafe { VirtualQueryEx(self.handle, addr as *const c_void, &mut info, std::mem::size_of_val(&info)) };
            if got == 0 {
                break;
            }
            let region_start = info.BaseAddress as usize;
            let region_end = (region_start + info.RegionSize).min(end);
            let executable =
                info.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY) != 0;
            let readable = info.State == MEM_COMMIT && info.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0;
            if readable && (executable || !executable_only) {
                let mut pos = addr.max(region_start);
                while pos < region_end {
                    // Blocks overlap by pattern.len() - 1 to catch occurrences straddling a boundary.
                    let want = (region_end - pos).min(chunk.len());
                    if self.read_bytes(pos, &mut chunk[..want]).is_ok() {
                        hits.extend(positions(&chunk[..want], pattern).map(|i| pos + i));
                    }
                    if want < chunk.len() {
                        break;
                    }
                    pos += SCAN_CHUNK + 1;
                }
            }
            addr = region_start + info.RegionSize;
        }
        hits
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

fn positions<'a>(haystack: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    haystack.windows(needle.len()).enumerate().filter(move |(_, w)| *w == needle).map(|(i, _)| i)
}

fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn this_process() -> Process {
        Process::open(std::process::id(), "self".into(), Access::Memory).unwrap()
    }

    #[test]
    fn finds_itself_by_name() {
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_name().unwrap().to_string_lossy().to_uppercase();
        let (pid, _) = find(&[name]).unwrap();
        // Test binaries run in parallel share the same name: we only check that a process is found.
        assert_ne!(pid, 0);
        assert!(find(&["definitely-not-running.exe".into()]).is_none());
    }

    #[test]
    fn reads_and_writes_memory() {
        let p = this_process();
        let value = Box::new(1234i32);
        let addr = &*value as *const i32 as usize;
        assert_eq!(p.read_i32(addr).unwrap(), 1234);
        p.write_bytes(addr, &5678i32.to_le_bytes()).unwrap();
        assert_eq!(unsafe { std::ptr::read_volatile(addr as *const i32) }, 5678);
        assert!(p.read_i32(8).is_err());
    }

    #[test]
    fn scans_patterns_across_chunk_boundaries() {
        let p = this_process();
        let needle = *b"\x13\x37iw-helper-scan\xC0\xDE";
        let mut heap = vec![0u8; SCAN_CHUNK * 3];
        // One occurrence in the middle of a block, one straddling the first block boundary.
        let offsets = [4096usize, SCAN_CHUNK - 5];
        for &o in &offsets {
            heap[o..o + needle.len()].copy_from_slice(&needle);
        }
        let start = heap.as_ptr() as usize;
        let hits = p.find_pattern(start, start + heap.len(), &needle, false);
        assert_eq!(hits, offsets.map(|o| start + o));
        assert!(p.find_pattern(start, start + heap.len(), &needle, true).is_empty());
    }

    #[test]
    fn scheduling_only_handle_cannot_touch_memory() {
        let p = Process::open(std::process::id(), "self".into(), Access::SchedulingOnly).unwrap();
        let value = 1i32;
        assert!(p.read_i32(&value as *const i32 as usize).is_err());
        assert!(p.is_running());
        assert_ne!(p.affinity().unwrap(), 0);
    }

    #[test]
    fn read_only_handle_cannot_write() {
        let p = Process::open(std::process::id(), "self".into(), Access::MemoryRead).unwrap();
        let value = Box::new(7i32);
        let addr = &*value as *const i32 as usize;
        assert_eq!(p.read_i32(addr).unwrap(), 7);
        assert!(p.write_bytes(addr, &[1]).is_err());
        assert!(p.set_affinity(1).is_err());
    }
}
