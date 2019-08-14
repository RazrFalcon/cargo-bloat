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

pub(crate) fn parse(data: &[u8], byte_order: ByteOrder) -> Vec<SymbolData> {
    let mut s = Stream::new(&data[16..], byte_order);
    s.skip::<elf::Half>(); // type
    s.skip::<elf::Half>(); // machine
    s.skip::<elf::Word>(); // version
    s.skip::<elf::Address>(); // entry
    s.skip::<elf::Offset>(); // phoff
    let section_offset = s.read::<elf::Offset>() as usize;
    s.skip::<elf::Word>(); // flags
    s.skip::<elf::Half>(); // ehsize
    s.skip::<elf::Half>(); // phentsize
    s.skip::<elf::Half>(); // phnum
    s.skip::<elf::Half>(); // shentsize
    let sections_count: elf::Half = s.read();
    s.skip::<elf::Half>(); // shstrndx

    let s = Stream::new(&data[section_offset..], byte_order);
    parse_section_header(data, s, sections_count)
}

struct Section {
    kind: u32,
    link: usize,
    range: Range<usize>,
    entries: usize,
}

fn parse_section_header(data: &[u8], mut s: Stream, count: u16) -> Vec<SymbolData> {
    let mut sections = Vec::with_capacity(count as usize);
    for _ in 0..count {
        s.skip::<elf::Word>(); // name
        let kind: elf::Word = s.read();
        s.skip::<elf::Word>(); // flags
        s.skip::<elf::Address>(); // addr
        let offset = s.read::<elf::Offset>() as usize;
        let size = s.read::<elf::Word>() as usize;
        let link = s.read::<elf::Word>() as usize;
        s.skip::<elf::Word>(); // info
        s.skip::<elf::Word>(); // addralign
        let entry_size = s.read::<elf::Word>() as usize;

        let entries = if entry_size == 0 { 0 } else { size / entry_size };

        sections.push(Section {
            kind,
            link,
            range: offset..(offset + size),
            entries,
        });
    }

    let section = match sections.iter().find(|v| v.kind == section_type::SYMBOL_TABLE) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let linked_section = match sections.get(section.link) {
        Some(v) => v,
        None => return Vec::new(),
    };

    if linked_section.kind != section_type::STRING_TABLE {
        return Vec::new();
    }

    let strings = &data[linked_section.range.clone()];
    let s = Stream::new(&data[section.range.clone()], s.byte_order());
    parse_symbols(s, section.entries, strings)
}

fn parse_symbols(mut s: Stream, count: usize, strings: &[u8]) -> Vec<SymbolData> {
    let mut symbols = Vec::with_capacity(count);
    while !s.at_end() {
        // Note: the order of fields in 32 and 64 bit ELF is different.
        let name_offset = s.read::<elf::Word>() as usize;
        s.skip::<elf::Address>(); // value
        let size: elf::Word = s.read();
        let info: u8 = s.read();
        s.skip::<u8>(); // other
        s.skip::<elf::Half>(); // shndx

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
                size: size as u64,
            });
        }
    }

    symbols
}
