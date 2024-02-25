//! Copyright © 2024 ChefKiss Inc. Licensed under the Thou Shalt Not Profit License version 1.5.
//! See LICENSE for details.

use binaryninja::{
    binaryview::{BinaryView, BinaryViewBase, BinaryViewExt},
    command::{register_for_address, AddressCommand},
    symbol::Symbol,
    Endianness,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FirmwareType {
    Gc,
    Sdma,
}

impl FirmwareType {
    fn size_field_off(self) -> u64 {
        match self {
            Self::Gc => 0xC,
            Self::Sdma => 0x8,
        }
    }

    fn off_field_off(self) -> u64 {
        match self {
            Self::Gc => 0x20,
            Self::Sdma => 0x10,
        }
    }
}

struct ExtractorCommand(FirmwareType);

impl ExtractorCommand {
    fn new(ty: FirmwareType) -> Self {
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
        full_name
            .as_str()
            .strip_prefix('_')
            .unwrap_or_else(|| full_name.as_str())
            .to_owned()
    }

    fn read_fw_info_of_sym(&self, view: &BinaryView, offset: u64) -> Option<(String, u64, u32)> {
        let (fw_name, address) = view
            .symbol_by_address(offset)
            .map(|v| (Self::sym_to_fw_name(&v), v.address()))
            .unwrap_or_else(|_| (format!("data_{offset:X}"), offset));
        self.read_fw_info(view, address)
            .map(|(fw_off, fw_size)| (fw_name, fw_off, fw_size))
    }
}

impl AddressCommand for ExtractorCommand {
    fn valid(&self, view: &BinaryView, addr: u64) -> bool {
        let Some((_, fw_off, fw_size)) = self.read_fw_info_of_sym(view, addr) else {
            return false;
        };
        view.offset_valid(fw_off) && view.offset_valid(fw_off + u64::from(fw_size))
    }

    fn action(&self, view: &BinaryView, addr: u64) {
        let Some((name, fw_off, fw_size)) = self.read_fw_info_of_sym(view, addr) else {
            return;
        };
        let data = view.read_vec(fw_off, fw_size.try_into().unwrap());
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{name}.bin"))
            .set_title(format!("Save {name}"))
            .save_file()
        else {
            return;
        };
        let Err(e) = std::fs::write(path, data) else {
            return;
        };
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("Whoops")
            .set_description(format!("File was not saved: {e}"))
            .set_buttons(rfd::MessageButtons::OkCustom("Well, shit".into()))
            .show();
    }
}

#[no_mangle]
pub extern "C" fn CorePluginInit() -> bool {
    register_for_address(
        "ChefKiss\\Extract GC firmware",
        "",
        ExtractorCommand::new(FirmwareType::Gc),
    );
    register_for_address(
        "ChefKiss\\Extract SDMA firmware",
        "",
        ExtractorCommand::new(FirmwareType::Sdma),
    );
    true
}
