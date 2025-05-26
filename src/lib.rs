// Copyright © 2024-2025 ChefKiss. Licensed under the Thou Shalt Not Profit License version 1.5.
// See LICENSE for details.

#![warn(clippy::nursery)]

use binaryninja::{
    Endianness,
    binary_view::{BinaryView, BinaryViewBase, BinaryViewExt},
    command::{AddressCommand, register_command_for_address},
    interaction::{MessageBoxButtonSet, MessageBoxIcon, get_save_filename_input, show_message_box},
    symbol::Symbol,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FirmwareType {
    Gc,
    Sdma,
}

impl FirmwareType {
    const fn size_field_off(self) -> u64 {
        match self {
            Self::Gc => 0xC,
            Self::Sdma => 0x8,
        }
    }

    const fn off_field_off(self) -> u64 {
        match self {
            Self::Gc => 0x20,
            Self::Sdma => 0x10,
        }
    }
}

struct ExtractorCommand(FirmwareType);

impl ExtractorCommand {
    const fn new(ty: FirmwareType) -> Self {
        Self(ty)
    }

    fn read_fw_size(&self, view: &BinaryView, offset: u64) -> Option<u32> {
        let data = view.read_vec(offset + self.0.size_field_off(), 4);
        Some(match view.default_endianness() {
            Endianness::LittleEndian => u32::from_le_bytes(data.as_slice().try_into().ok()?),
            Endianness::BigEndian => u32::from_be_bytes(data.as_slice().try_into().ok()?),
        })
    }

    fn read_fw_off(&self, view: &BinaryView, offset: u64) -> Option<u64> {
        let data = view.read_vec(offset + self.0.off_field_off(), view.address_size());
        Some(match view.default_endianness() {
            Endianness::LittleEndian => u64::from_le_bytes(data.as_slice().try_into().ok()?),
            Endianness::BigEndian => u64::from_be_bytes(data.as_slice().try_into().ok()?),
        })
    }

    fn read_fw_info(&self, view: &BinaryView, offset: u64) -> Option<(u64, u32)> {
        self.read_fw_off(view, offset)
            .and_then(|fw_off| self.read_fw_size(view, offset).map(|size| (fw_off, size)))
    }

    fn sym_to_fw_name(sym: &Symbol) -> String {
        let full_name = sym.full_name();
        let full_name = full_name.to_string_lossy();
        full_name
            .strip_prefix('_')
            .unwrap_or_else(|| &full_name)
            .to_owned()
    }

    fn fw_info_addr(view: &BinaryView, offset: u64) -> u64 {
        view.symbol_by_address(offset)
            .map(|v| v.address())
            .unwrap_or(offset)
    }

    fn read_fw_info_of_sym(&self, view: &BinaryView, offset: u64) -> Option<(String, u64, u32)> {
        let (fw_name, address) = view
            .symbol_by_address(offset)
            .map(|v| (Self::sym_to_fw_name(&v), v.address()))
            .unwrap_or_else(|| (format!("data_{offset:X}"), offset));
        self.read_fw_info(view, address)
            .map(|(fw_off, fw_size)| (fw_name, fw_off, fw_size))
    }
}

impl AddressCommand for ExtractorCommand {
    fn valid(&self, view: &BinaryView, addr: u64) -> bool {
        let Some((fw_off, fw_size)) = self.read_fw_info(view, Self::fw_info_addr(view, addr))
        else {
            return false;
        };
        view.offset_valid(fw_off) && view.offset_valid(fw_off + u64::from(fw_size))
    }

    fn action(&self, view: &BinaryView, addr: u64) {
        let Some((name, fw_off, fw_size)) = self.read_fw_info_of_sym(view, addr) else {
            return;
        };
        let data = view.read_vec(fw_off, fw_size.try_into().unwrap());
        let Some(path) =
            get_save_filename_input(&format!("Save {name}"), "bin", &format!("{name}.bin"))
        else {
            return;
        };
        let Err(e) = std::fs::write(path, data) else {
            return;
        };
        show_message_box(
            "Whoops",
            &format!("File was not saved: {e}"),
            MessageBoxButtonSet::OKButtonSet,
            MessageBoxIcon::ErrorIcon,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn CorePluginInit() -> bool {
    register_command_for_address(
        "Extract GC firmware",
        "",
        ExtractorCommand::new(FirmwareType::Gc),
    );
    register_command_for_address(
        "Extract SDMA firmware",
        "",
        ExtractorCommand::new(FirmwareType::Sdma),
    );
    true
}
