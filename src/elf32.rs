use std::ops::Range;

use crate::SymbolData;
use crate::parser::*;

mod elf {
    pub type Address = u32;
    pub type Offset = u32;
    pub type Half = u16;
    pub type Word = u32;
}

mod section_type {
    pub const SYMBOL_TABLE: super::elf::Word = 2;
    pub const STRING_TABLE: super::elf::Word = 3;
}

pub fn parse(data: &[u8], byte_order: ByteOrder) -> (Vec<SymbolData>, u64) {
    let mut s = Stream::new(&data[16..], byte_order);
    s.skip::<elf::Half>(); // type
    s.skip::<elf::Half>(); // machine
    s.skip::<elf::Word>(); // version
    s.skip::<elf::Address>(); // entry
    s.skip::<elf::Offset>(); // phoff
    let section_offset = s.read::<elf::Offset>() as usize; // shoff
    s.skip::<elf::Word>(); // flags
    s.skip::<elf::Half>(); // ehsize
    s.skip::<elf::Half>(); // phentsize
    s.skip::<elf::Half>(); // phnum
    s.skip::<elf::Half>(); // shentsize
    let sections_count: elf::Half = s.read(); // shnum
    let section_name_strings_index: elf::Half = s.read(); // shstrndx

    let s = Stream::new(&data[section_offset..], byte_order);
    match parse_section_header(data, s, sections_count, section_name_strings_index) {
        Some(v) => v,
        None => (Vec::new(), 0),
    }
}

#[derive(Clone, Copy)]
struct Section {
    index: u16,
    name: u32,
    kind: u32,
    link: usize,
    offset: u32,
    size: u32,
    entries: usize,
}

impl Section {
    fn range(&self) -> Range<usize> {
        self.offset as usize .. (self.offset as usize + self.size as usize)
    }
}

fn parse_section_header(
    data: &[u8],
    mut s: Stream,
    count: u16,
    section_name_strings_index: u16,
) -> Option<(Vec<SymbolData>, u64)> {
    let mut sections = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let name: elf::Word = s.read();
        let kind: elf::Word = s.read();
        s.skip::<elf::Word>(); // flags
        s.skip::<elf::Address>(); // addr
        let offset = s.read::<elf::Offset>();
        let size = s.read::<elf::Word>();
        let link = s.read::<elf::Word>() as usize;
        s.skip::<elf::Word>(); // info
        s.skip::<elf::Word>(); // addralign
        let entry_size = s.read::<elf::Word>();

        let entries = if entry_size == 0 { 0 } else { size / entry_size } as usize;

        sections.push(Section {
            index: sections.len() as u16,
            name,
            kind,
            link,
            offset,
            size,
            entries,
        });
    }

    let section_name_strings = &data[sections[section_name_strings_index as usize].range()];
    let text_section = sections.iter().find(|s| {
        parse_null_string(section_name_strings, s.name as usize) == Some(".text")
    }).cloned()?;

    let symbols_section = sections.iter().find(|v| v.kind == section_type::SYMBOL_TABLE)?;

    let linked_section = sections.get(symbols_section.link)?;
    if linked_section.kind != section_type::STRING_TABLE {
        return None;
    }

    let strings = &data[linked_section.range()];
    let s = Stream::new(&data[symbols_section.range()], s.byte_order());
    let symbols = parse_symbols(s, symbols_section.entries, strings, text_section);
    Some((symbols, text_section.size as u64))
}

fn parse_symbols(
    mut s: Stream,
    count: usize,
    strings: &[u8],
    text_section: Section,
) -> Vec<SymbolData> {
    let mut symbols = Vec::with_capacity(count);
    while !s.at_end() {
        // Note: the order of fields in 32 and 64 bit ELF is different.
        let name_offset = s.read::<elf::Word>() as usize;
        let value: elf::Address = s.read();
        let size: elf::Word = s.read();
        let info: u8 = s.read();
        s.skip::<u8>(); // other
        let shndx: elf::Half = s.read();

        if shndx != text_section.index {
            continue;
        }

        // Ignore symbols with zero size.
        if size == 0 {
            continue;
        }

        // Ignore symbols without a name.
        if name_offset == 0 {
            continue;
        }

        // Ignore symbols that aren't functions.
        const STT_FUNC: u8 = 2;
        let kind = info & 0xf;
        if kind != STT_FUNC {
            continue;
        }

        if let Some(s) = parse_null_string(strings, name_offset) {
            symbols.push(SymbolData {
                name: crate::demangle::SymbolName::demangle(s),
                address: value as u64,
                size: size as u64,
            });
        }
    }

    symbols
}
