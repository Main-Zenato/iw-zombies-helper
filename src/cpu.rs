use std::io;
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};

/// One affinity mask per physical core (all of its logical processors), sorted by first logical processor.
///
/// Only processor group 0 is considered, which covers any machine with 64 logical processors or fewer.
pub fn physical_cores() -> io::Result<Vec<usize>> {
    let mut len = 0u32;
    unsafe { GetLogicalProcessorInformationEx(RelationProcessorCore, std::ptr::null_mut(), &mut len) };
    if len == 0 {
        return Err(io::Error::last_os_error());
    }
    // A u64 buffer keeps the alignment the API needs for the structures it writes there.
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    let ok = unsafe { GetLogicalProcessorInformationEx(RelationProcessorCore, buf.as_mut_ptr().cast(), &mut len) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut cores = Vec::new();
    let base = buf.as_ptr().cast::<u8>();
    let mut offset = 0usize;
    while offset < len as usize {
        let info = unsafe { &*base.add(offset).cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>() };
        if info.Relationship == RelationProcessorCore {
            let group = unsafe { info.Anonymous.Processor.GroupMask[0] };
            if group.Group == 0 && group.Mask != 0 {
                cores.push(group.Mask);
            }
        }
        offset += info.Size as usize;
    }
    cores.sort_by_key(|mask| mask.trailing_zeros());
    Ok(cores)
}

/// Builds the process affinity mask: the first `max_cores` physical cores (0 = all), with either
/// every logical processor of each core (`smt`) or only the first one.
pub fn build_mask(cores: &[usize], smt: bool, max_cores: usize) -> usize {
    let take = if max_cores == 0 { cores.len() } else { max_cores.min(cores.len()) };
    cores[..take].iter().fold(0, |acc, &core| acc | if smt { core } else { core.isolate_lowest_one() })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4 cores / 8 threads, sibling threads adjacent: 0b11, 0b1100, ...
    const CORES: [usize; 4] = [0x03, 0x0C, 0x30, 0xC0];

    #[test]
    fn smt_off_keeps_one_thread_per_core() {
        assert_eq!(build_mask(&CORES, false, 0), 0b0101_0101);
    }

    #[test]
    fn smt_on_keeps_everything() {
        assert_eq!(build_mask(&CORES, true, 0), 0xFF);
    }

    #[test]
    fn max_cores_limits_and_clamps() {
        assert_eq!(build_mask(&CORES, true, 2), 0x0F);
        assert_eq!(build_mask(&CORES, false, 2), 0b0101);
        assert_eq!(build_mask(&CORES, false, 99), 0b0101_0101);
    }

    #[test]
    fn real_topology_is_consistent() {
        let cores = physical_cores().unwrap();
        assert!(!cores.is_empty());
        let all = build_mask(&cores, true, 0);
        let one_per_core = build_mask(&cores, false, 0);
        assert_eq!(one_per_core.count_ones() as usize, cores.len());
        assert_eq!(one_per_core & all, one_per_core);
    }
}
