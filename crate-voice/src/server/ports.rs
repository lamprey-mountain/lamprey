use std::collections::HashSet;

use crate::prelude::*;

/// allocates fixed udp ports from a configured range for shard media sockets
pub struct PortAllocator {
    min: u16,
    max: u16,
    next: u16,
    reserved: HashSet<u16>,
    used: HashSet<u16>,
}

impl PortAllocator {
    pub fn new(
        min: u16,
        max: u16,
        workers: usize,
        reserved: impl IntoIterator<Item = u16>,
    ) -> Result<Self> {
        if min < 1024 {
            return Err(Error::Config(format!(
                "rtp_port_min {min} is a privileged port"
            )));
        }

        if min > max {
            return Err(Error::Config(format!(
                "rtp_port_min {min} is greater than rtp_port_max {max}"
            )));
        }

        let reserved: HashSet<u16> = reserved.into_iter().filter(|p| *p != 0).collect();

        let available = (min..=max).filter(|p| !reserved.contains(p)).count();
        if available < workers {
            return Err(Error::Config(format!(
                "rtp port range {min}-{max} has {available} usable ports, but {workers} workers need one each"
            )));
        }

        Ok(Self {
            min,
            max,
            next: min,
            reserved,
            used: HashSet::new(),
        })
    }

    /// all candidate ports, starting after the last allocated port and wrapping once
    pub fn candidates(&self) -> Vec<u16> {
        if self.min > self.max {
            return Vec::new();
        }

        let width = (self.max - self.min) as usize + 1;
        let start = (self.next - self.min) as usize;

        (0..width)
            .map(|i| self.min + ((start + i) % width) as u16)
            .filter(|p| !self.reserved.contains(p) && !self.used.contains(p))
            .collect()
    }

    /// advance past a port that was successfully bound
    pub fn mark_used(&mut self, port: u16) {
        self.used.insert(port);
        self.next = if port >= self.max { self.min } else { port + 1 };
    }

    /// release a port after its shard exits
    pub fn release(&mut self, port: u16) {
        self.used.remove(&port);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_sequentially() {
        let mut ports = PortAllocator::new(56000, 56002, 3, []).unwrap();
        assert_eq!(ports.candidates(), vec![56000, 56001, 56002]);

        ports.mark_used(56000);
        assert_eq!(ports.candidates(), vec![56001, 56002]);
    }

    #[test]
    fn skips_reserved() {
        let ports = PortAllocator::new(56000, 56003, 3, [56001]).unwrap();
        assert_eq!(ports.candidates(), vec![56000, 56002, 56003]);
    }

    #[test]
    fn wraps_after_last_port() {
        let mut ports = PortAllocator::new(56000, 56002, 2, []).unwrap();
        ports.mark_used(56002);
        assert_eq!(ports.candidates(), vec![56000, 56001]);

        ports.mark_used(56000);
        assert_eq!(ports.candidates(), vec![56001]);
    }

    #[test]
    fn releases_ports() {
        let mut ports = PortAllocator::new(56000, 56001, 2, []).unwrap();
        ports.mark_used(56000);
        ports.mark_used(56001);
        assert!(ports.candidates().is_empty());

        ports.release(56000);
        assert_eq!(ports.candidates(), vec![56000]);
    }

    #[test]
    fn rejects_small_ranges() {
        assert!(PortAllocator::new(56000, 56001, 3, []).is_err());
        assert!(PortAllocator::new(56001, 56000, 1, []).is_err());
        assert!(PortAllocator::new(100, 200, 1, []).is_err());
        assert!(PortAllocator::new(56000, 56001, 2, [56000, 56001]).is_err());
    }
}
