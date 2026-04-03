use std::net::Ipv4Addr;

use anyhow::{Result, bail};

/// Deterministically allocate guest IPs, TAP names, and MAC addresses.
///
/// The first version keeps this logic intentionally boring: one /24 network,
/// host at `.1`, guests starting at `.2`, and a locally administered MAC range.
#[derive(Debug, Clone)]
pub struct NetworkPlan {
    base: Ipv4Addr,
    prefix_len: u8,
}

impl NetworkPlan {
    pub fn new(base: Ipv4Addr, prefix_len: u8) -> Result<Self> {
        if prefix_len > 30 {
            bail!("prefix length {prefix_len} is too small for the MVP");
        }

        Ok(Self { base, prefix_len })
    }

    pub fn gateway(&self) -> Result<Ipv4Addr> {
        self.offset_ip(1)
    }

    pub fn guest_cidr(&self, index: usize) -> Result<String> {
        Ok(format!("{}/{}", self.guest_ip(index)?, self.prefix_len))
    }

    pub fn guest_ip(&self, index: usize) -> Result<Ipv4Addr> {
        self.offset_ip(index + 2)
    }

    pub fn tap_name(&self, prefix: &str, index: usize) -> Result<String> {
        let name = format!("{prefix}{index}");
        if name.len() > 15 {
            bail!("tap name exceeds Linux interface limit: {name}");
        }

        Ok(name)
    }

    pub fn mac_address(&self, index: usize) -> Result<String> {
        if index > u16::MAX as usize {
            bail!("too many VMs for the simple MAC allocator");
        }

        let hi = ((index >> 8) & 0xff) as u8;
        let lo = (index & 0xff) as u8;

        // 02:xx marks the address as locally administered and unicast.
        Ok(format!("02:fc:00:00:{hi:02x}:{lo:02x}"))
    }

    fn offset_ip(&self, offset: usize) -> Result<Ipv4Addr> {
        let base = u32::from(self.base);
        let candidate = base
            .checked_add(offset as u32)
            .ok_or_else(|| anyhow::anyhow!("ip overflow for offset {offset}"))?;

        Ok(Ipv4Addr::from(candidate))
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::NetworkPlan;

    #[test]
    fn allocates_expected_addresses() {
        let plan = NetworkPlan::new(Ipv4Addr::new(192, 168, 100, 0), 24).unwrap();
        assert_eq!(plan.gateway().unwrap(), Ipv4Addr::new(192, 168, 100, 1));
        assert_eq!(plan.guest_ip(0).unwrap(), Ipv4Addr::new(192, 168, 100, 2));
        assert_eq!(plan.guest_ip(3).unwrap(), Ipv4Addr::new(192, 168, 100, 5));
    }

    #[test]
    fn keeps_tap_names_short() {
        let plan = NetworkPlan::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap();
        assert_eq!(plan.tap_name("mvm", 12).unwrap(), "mvm12");
        assert!(plan.tap_name("this-prefix-is-too-long", 1).is_err());
    }

    #[test]
    fn generates_mac_addresses() {
        let plan = NetworkPlan::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap();
        assert_eq!(plan.mac_address(1).unwrap(), "02:fc:00:00:00:01");
        assert_eq!(plan.mac_address(257).unwrap(), "02:fc:00:00:01:01");
    }
}

