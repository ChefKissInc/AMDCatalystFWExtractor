//! Copyright © 2024 ChefKiss Inc. Licensed under the Thou Shalt Not Profit License version 1.5.
//! See LICENSE for details.

use binaryninja::{
    binaryview::{BinaryView, BinaryViewBase, BinaryViewExt},
    command::{register_for_address, AddressCommand},
    Endianness,
};

struct ExtractorCommand;

impl ExtractorCommand {
    fn read_fw_size(view: &BinaryView, offset: u64) -> Option<u32> {
        let data = view.read_vec(offset + 0xC, 4);
        Some(match view.default_endianness() {
            Endianness::LittleEndian => u32::from_le_bytes(data.as_slice().try_into().ok()?),
            Endianness::BigEndian => u32::from_be_bytes(data.as_slice().try_into().ok()?),
        })
    }

    fn read_fw_off(view: &BinaryView, offset: u64) -> Option<u64> {
        let data = view.read_vec(offset + 0x20, view.address_size());
        Some(match view.default_endianness() {
            Endianness::LittleEndian => u64::from_le_bytes(data.as_slice().try_into().ok()?),
            Endianness::BigEndian => u64::from_be_bytes(data.as_slice().try_into().ok()?),
        })
    }

    fn read_fw_info(view: &BinaryView, offset: u64) -> Option<(u64, u32)> {
        Self::read_fw_off(view, offset)
            .and_then(|fw_off| Self::read_fw_size(view, offset).map(|size| (fw_off, size)))
    }

    fn read_fw_info_of_sym(view: &BinaryView, offset: u64) -> Option<(String, u64, u32)> {
        view.symbol_by_address(offset).ok().and_then(|v| {
            Self::read_fw_info(view, v.address()).map(|(fw_off, fw_size)| {
                (
                    v.full_name()
                        .as_str()
                        .strip_prefix('_')
                        .map(String::from)
                        .unwrap_or_else(|| v.full_name().as_str().to_owned()),
                    fw_off,
                    fw_size,
                )
            })
        })
    }
}

impl AddressCommand for ExtractorCommand {
    fn valid(&self, view: &BinaryView, addr: u64) -> bool {
        let Some((_, fw_off, fw_size)) = Self::read_fw_info_of_sym(view, addr) else {
            return false;
        };
        view.offset_valid(fw_off) && view.offset_valid(fw_off + u64::from(fw_size))
    }

    fn action(&self, view: &BinaryView, addr: u64) {
        let Some((name, fw_off, fw_size)) = Self::read_fw_info_of_sym(view, addr) else {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Well")
                .set_description("There's no symbol here, bozo")
                .set_buttons(rfd::MessageButtons::OkCustom("Oh".into()))
                .show();
            return;
        };
        let data = view.read_vec(fw_off, fw_size.try_into().unwrap());
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{name}.bin"))
            .set_title(format!("Save {name}"))
            .save_file()
        else {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Info)
                .set_title("Well")
                .set_description("Ok then")
                .set_buttons(rfd::MessageButtons::OkCustom("Yeah".into()))
                .show();
            return;
        };
        match std::fs::write(path, data) {
            Ok(_) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_title("Nice")
                    .set_description("File was saved")
                    .set_buttons(rfd::MessageButtons::OkCustom("Nice".into()))
                    .show();
            }
            Err(e) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_title("Whoops")
                    .set_description(format!("File was not saved: {e}"))
                    .set_buttons(rfd::MessageButtons::OkCustom("Well, shit".into()))
                    .show();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn CorePluginInit() -> bool {
    register_for_address(
        "ChefKiss\\Extract AMD Catalyst Firmware",
        "",
        ExtractorCommand,
    );
    true
}
