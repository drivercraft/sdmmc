#![allow(dead_code)]

use aarch64_cpu::registers::SCTLR_EL3::C;

use super::EMmcHost;

impl EMmcHost {
    // Read a 32-bit register
    pub fn read_reg(&self, offset: u32) -> u32 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u32) }
    }

    // Read a 16-bit register
    pub fn read_reg16(&self, offset: u32) -> u16 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u16) }
    }

    // Read an 8-bit register
    pub fn read_reg8(&self, offset: u32) -> u8 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u8) }
    }

    // Write a 32-bit register
    pub fn write_reg(&self, offset: u32, value: u32) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u32, value) }
    }

    // Write a 16-bit register
    pub fn write_reg16(&self, offset: u32, value: u16) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u16, value) }
    }

    // Write an 8-bit register
    pub fn write_reg8(&self, offset: u32, value: u8) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u8, value) }
    }
}

struct CSDRegister {
    csd_structure: u8,
    spec_version: u8,
    device_size: u16,
    devoce_size_mult: u8,
    read_bl_len: u8,
}

impl CSDRegister {
    fn new(csd: &[u32; 4]) -> Self {
        let csd_structure = (csd[3] >> 30) as u8;
        let spec_version = ((csd[3] >> 26) & 0xF) as u8;
        let device_size = (((csd[2] & 0x3FF) << 2) | ((csd[1] >> 30) & 0x3)) as u16;
        let devoce_size_mult = ((csd[1] >> 15) & 0x7) as u8;
        let read_bl_len = ((csd[2] >> 8) & 0xF) as u8;
        CSDRegister {
            csd_structure,
            spec_version,
            device_size,
            devoce_size_mult,
            read_bl_len,
        }
    }
}