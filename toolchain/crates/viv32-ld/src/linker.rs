use std::{fs::File, io::Write, path::PathBuf};

use viv32_vo::{Bss, ObjectFile, Relocation, RelocationBase, RelocationSign, Symbol};

use crate::LinkerError;

#[derive(Debug)]
pub struct Linker {
    text: Vec<u8>,
    rodata: Vec<u8>,
    data: Vec<u8>,
    bss: Vec<Bss>,
    symbols: Vec<Symbol>,
    relocations: Vec<Relocation>,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            text: Vec::new(),
            rodata: Vec::new(),
            data: Vec::new(),
            bss: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
        }
    }

    pub fn add(&mut self, file: File) -> Result<(), LinkerError> {
        let current = ObjectFile::read(file)?;
        let offset = self.text.len();

        // Add bytes and pad if necessary
        self.text.extend_from_slice(&current.bytes);
        self.text.resize(word_align(self.text.len()), 0);

        // Add bss.
        for bss in current.bss {
            self.check_duplicate_label(&bss.name)?;
            self.bss.push(bss.clone());
        }

        // Add adjusted symbols.
        for symbol in current.symbols {
            self.check_duplicate_label(&symbol.name)?;
            let mut symbol = symbol.clone();
            symbol.offset += offset as u32;
            self.symbols.push(symbol);
        }

        // Add adjusted relocations.
        for relocation in current.relocations {
            let mut relocation = relocation.clone();
            relocation.patch_offset += offset as u32;
            self.relocations.push(relocation);
        }

        Ok(())
    }

    pub fn link(&mut self, path: &PathBuf) -> Result<(), LinkerError> {
        self.map_bss_to_symbol()?;
        self.apply_relocations()?;
        let mut file = File::create(path)?;
        file.write_all(&self.text)?;
        file.write_all(&self.rodata)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    fn apply_relocations(&mut self) -> Result<(), LinkerError> {
        for relocation in &self.relocations {
            let symbol = self.get_symbol(&relocation.symbol)?;
            let patch = relocation.patch_offset as usize;
            let target_addr = calculate_target_addr(&relocation, &symbol);
            let target_addr = target_shift(target_addr, &relocation)?;
            let (field, mask) = target_field_mask(target_addr, &relocation);

            let data = u32::from_be_bytes(
                self.text[patch..(patch + 4)]
                    .try_into()
                    .expect("Should have existed"),
            );
            let data: u32 = (data & mask) | field;
            self.text[patch..(patch + 4)].copy_from_slice(&data.to_be_bytes());
        }
        Ok(())
    }

    fn get_symbol(&self, name: &str) -> Result<Symbol, LinkerError> {
        let symbol = self.symbols.iter().find(|s| s.name == name);
        symbol.cloned().ok_or(LinkerError::UnknownLabel(format!(
            "Unable to link undeclared label: {}",
            name.to_owned()
        )))
    }

    fn map_bss_to_symbol(&mut self) -> Result<(), LinkerError> {
        // Calculate offset, consider section alignment.
        let mut offset = word_align(self.text.len());
        offset = word_align(offset + self.rodata.len());
        offset = word_align(offset + self.data.len());

        for bss in &self.bss {
            offset = word_align(offset);
            self.symbols.push(Symbol {
                offset: offset as u32,
                name: bss.name.to_owned(),
            });
            offset += bss.size as usize;
        }
        Ok(())
    }

    fn check_duplicate_label(&self, label: &str) -> Result<(), LinkerError> {
        if self.symbols.iter().any(|s| s.name == label) || self.bss.iter().any(|b| b.name == label)
        {
            return Err(LinkerError::DuplicateLabel(format!(
                "Duplicated Symbol: {}",
                label,
            )));
        }
        Ok(())
    }
}

fn target_field_mask(target: i64, relocation: &Relocation) -> (u32, u32) {
    let mask: i64 = (1_i64 << relocation.width) - 1;
    let field = ((target & mask) << relocation.field_shift) as u32;
    let mask = (mask as u32) << relocation.field_shift;
    (field, !mask)
}

fn calculate_target_addr(relocation: &Relocation, symbol: &Symbol) -> i64 {
    let mut target = symbol.offset as i64 + relocation.addend as i64;
    if relocation.base == RelocationBase::Relative {
        target = target - relocation.patch_offset as i64;
    }
    target
}

fn target_shift(target: i64, relocation: &Relocation) -> Result<i64, LinkerError> {
    use RelocationSign::{Signed, Unsigned};

    let target = target >> relocation.value_shift;
    if relocation.bounds_check != 0 {
        let (min, max) = match relocation.sign {
            Unsigned => (0_i64, (1_i64 << relocation.width) - 1),
            Signed => (
                -(1_i64 << (relocation.width - 1)),
                (1_i64 << (relocation.width - 1)) - 1,
            ),
        };
        if target < min || target > max {
            return Err(LinkerError::InvalidRange(format!(
                "Invalid range for width {} detected: {} vs {}..{}",
                relocation.width, target, min, max
            )));
        }
    }
    Ok(target)
}

fn word_align(len: usize) -> usize {
    let ragged = len % 4;
    if ragged != 0 { len + (4 - ragged) } else { len }
}
