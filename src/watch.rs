//! Read-only watch of two structures of the host player, to see what the game changes there by
//! itself: their entity (`gentity_s`) and their client (`gclient_s`, which starts with the player state).
//!
//! It helps find a setting without guessing it. Example: the "Blind Spot"-style card makes the
//! player invisible to zombies for 20 seconds; the byte that goes from 0 to 1 when it is played,
//! then back to 0, is the one "no target" has to write.
//!
//! Most bytes change all the time (position, clocks). During a short learning period changes are
//! only counted; after that, only bytes that stayed quiet are reported, each a small number of
//! times.

use crate::process::Process;
use std::time::{Duration, Instant};

const G_ENTITIES: usize = 0x1_43D2_2610;
const ENTITY_SIZE: usize = 0x3F8;
const HOST_CLIENT: usize = 0x1_43CA_2060;
/// Watched part of `gclient_s`: its player state and session. Beyond that (up to 0x7218) are
/// position histories rewritten in a loop, which drown everything else.
const CLIENT_WATCHED: usize = 0x5400;
pub const LEARNING: Duration = Duration::from_secs(15);
/// Number of changes beyond which a byte is no longer reported.
const QUIET_LIMIT: u16 = 6;

struct Region {
    label: &'static str,
    addr: usize,
    last: Vec<u8>,
    changes: Vec<u16>,
}

impl Region {
    fn new(label: &'static str, addr: usize, len: usize) -> Self {
        Self { label, addr, last: Vec::new(), changes: vec![0; len] }
    }

    /// Reads the region again; returns the quiet byte runs that just changed.
    fn poll(&mut self, p: &Process, learning: bool) -> Vec<String> {
        let mut now = vec![0u8; self.changes.len()];
        if p.read_bytes(self.addr, &mut now).is_err() {
            return Vec::new();
        }
        let before = std::mem::replace(&mut self.last, now);
        if before.is_empty() {
            return Vec::new();
        }
        let mut reported = Vec::new();
        for (offset, count) in self.changes.iter_mut().enumerate() {
            if before[offset] != self.last[offset] {
                *count = count.saturating_add(1);
                if !learning && *count <= QUIET_LIMIT {
                    reported.push(offset);
                }
            }
        }
        runs(&reported)
            .into_iter()
            .map(|(start, end)| crate::lang::trs(
                format!("Watching, {} +{start:#05x}: {} -> {}", self.label, hex(&before[start..=end]), hex(&self.last[start..=end])),
                format!("Surveillance, {} +{start:#05x} : {} -> {}", self.label, hex(&before[start..=end]), hex(&self.last[start..=end])),
            ))
            .collect()
    }
}

/// Groups increasing offsets into runs of neighbouring bytes: (first, last).
fn runs(offsets: &[usize]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &offset in offsets {
        match out.last_mut() {
            Some((_, end)) if *end + 1 == offset => *end = offset,
            _ => out.push((offset, offset)),
        }
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// Watch in progress; it starts with its learning period.
pub struct Watch {
    started: Instant,
    announced: bool,
    regions: [Region; 2],
}

impl Default for Watch {
    fn default() -> Self {
        let regions = [Region::new("entity", G_ENTITIES, ENTITY_SIZE), Region::new("client", HOST_CLIENT, CLIENT_WATCHED)];
        Self { started: Instant::now(), announced: false, regions }
    }
}

impl Watch {
    /// To call a few times per second; returns the lines to log.
    pub fn poll(&mut self, p: &Process) -> Vec<String> {
        let learning = self.started.elapsed() < LEARNING;
        let mut lines = Vec::new();
        if !learning && !self.announced {
            self.announced = true;
            lines.push(crate::lang::tr("Watching ready: what the game now changes on the player will be logged.", "Surveillance prête : ce que le jeu change maintenant sur le joueur sera noté.").to_string());
        }
        for region in &mut self.regions {
            lines.extend(region.poll(p, learning));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Access;

    #[test]
    fn neighbouring_offsets_are_reported_together() {
        assert_eq!(runs(&[]), vec![]);
        assert_eq!(runs(&[4, 5, 6, 9, 20, 21]), vec![(4, 6), (9, 9), (20, 21)]);
    }

    #[test]
    fn quiet_bytes_are_reported_and_noisy_ones_are_muted() {
        let p = Process::open(std::process::id(), "self".into(), Access::Memory).unwrap();
        let memory = Box::new([0u8; 32]);
        let addr = memory.as_ptr() as usize;
        let mut region = Region::new("essai", addr, 32);
        assert!(region.poll(&p, true).is_empty(), "the first reading has nothing to compare");

        // Learning: byte 3 changes all the time, nothing is reported.
        for value in 1..=10u8 {
            p.write_bytes(addr + 3, &[value]).unwrap();
            assert!(region.poll(&p, true).is_empty());
        }
        // After that: the noisy byte stays silent, the quiet byte is reported with its neighbours.
        p.write_bytes(addr + 3, &[99]).unwrap();
        p.write_bytes(addr + 20, &[1, 2]).unwrap();
        assert_eq!(region.poll(&p, false), vec!["Watching, essai +0x014: 00 00 -> 01 02".to_string()]);
        assert!(region.poll(&p, false).is_empty(), "nothing changed since");
        p.write_bytes(addr + 20, &[0]).unwrap();
        assert_eq!(region.poll(&p, false), vec!["Watching, essai +0x014: 01 -> 00".to_string()]);
    }
}
