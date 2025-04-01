#![allow(dead_code)]

pub struct OCRRegister {

}

impl OCRRegister {
    pub fn from_bytes(_bytes: &[u32; 4]) -> Self {
        todo!()
    }
}

pub struct CIDRegister {
    manufacturer_id: u8,
    device: u8,
    application_id: u16,
    name: [u8; 6],
    revision: u8,
    serial_number: u32,
    manufacturing_date: u16,
    crc7_checksum: u8,
}

impl CIDRegister {
    pub fn from_bytes(_bytes: &[u32; 4]) -> Self {
        todo!()
    }
}

pub struct CSDRegister {
    csd_structure: u8,
    system_specification_version: u8,
    data_read_access_time1: u8,
    data_read_access_time2: u8,
    max_bus_clock_frequency: u8,
    device_command_classes: u8,
    partial_blocks_write_allowed: u8,
    write_block_misalignment: u8,
    read_block_misalignment: u8,
    dsr_implemented: u8,
    device_size: u16,
    max_read_current_min: u8,  
    max_read_current_max: u8, 
    max_write_current_min: u8,
    max_write_current_max: u8,
    device_size_multiplier: u8,
    erase_group_size: u8,
    erase_group_size_multiplier: u8,
    write_protect_group_size: u8,
    write_protect_group_enable: u8,
    manufacturer_default_ecc: u8,
    write_speed_factor: u8,
    max_write_data_block_length: u8,
    partial_blocks_for_write_allowed: u8,
    content_protect_application: u8,
    file_format_group: u8,
    copy_flags: u8,
    perm_write_protect: u8,
    temp_write_protect: u8,
    file_format: u8,
    ecc_code: u8,
    crc: u8,
}

impl CSDRegister {
    pub fn from_bytes(_bytes: &[u32; 4]) -> Self {
        todo!()
    }
}

pub struct ExtCSDRegister {
    
}

impl ExtCSDRegister {
    pub fn from_bytes(_bytes: &[u32; 4]) -> Self {
        todo!()
    }
}

pub struct  RCARegister {

}

pub struct DSRRegister {

}

pub struct QSR {

}

