use crate::SymbolData;
use crate::parser::*;

const LC_SYMTAB: u32 = 0x2;
const LC_SEGMENT_64: u32 = 0x19;

#[derive(Clone, Copy)]
struct Cmd {
    kind: u32,
    offset: usize,
}

#[derive(Clone, Copy)]
struct Section {
    address: u64,
    size: u64,
}

pub fn parse(data: &[u8]) -> (Vec<SymbolData>, u64) {
    let mut s = Stream::new(data, ByteOrder::LittleEndian);
    s.skip::<u32>(); // magic
    s.skip::<u32>(); // cputype
    s.skip::<u32>(); // cpusubtype
    s.skip::<u32>(); // filetype
    let number_of_commands: u32 = s.read();
    s.skip::<u32>(); // sizeofcmds
    s.skip::<u32>(); // flags
    s.skip::<u32>(); // reserved

    let mut commands = Vec::with_capacity(number_of_commands as usize);
    for _ in 0..number_of_commands {
        let cmd: u32 = s.read();
        let cmd_size: u32 = s.read();

        commands.push(Cmd {
            kind: cmd,
            offset: s.offset(),
        });

        // cmd_size is a size of a whole command data,
        // so we have to remove the header size first.
        s.skip_len(cmd_size as usize - 8);
    }

    let mut text_section = Section { address: 0, size: 0 };
    for cmd in &commands {
        if cmd.kind == LC_SEGMENT_64 {
            let mut s = Stream::new_at(data, cmd.offset, ByteOrder::LittleEndian);
            s.skip_len(16); // segname
            s.skip::<u64>(); // vmaddr
            s.skip::<u64>(); // vmsize
            s.skip::<u64>(); // fileoff
            s.skip::<u64>(); // filesize
            s.skip::<u32>(); // maxprot
            s.skip::<u32>(); // initprot
            let sections_count: u32 = s.read();
            s.skip::<u32>(); // flags

            for i in 0..sections_count {
                let section_name = parse_null_string(s.read_bytes(16), 0);
                let segment_name = parse_null_string(s.read_bytes(16), 0);
                let address: u64 = s.read();
                let size: u64 = s.read();
                s.skip::<u32>(); // offset
                s.skip::<u32>(); // align
                s.skip::<u32>(); // reloff
                s.skip::<u32>(); // nreloc
                s.skip::<u32>(); // flags
                s.skip_len(12); // padding

                if segment_name == Some("__TEXT") && section_name == Some("__text") {
                    text_section = Section { address, size };
                    assert_eq!(i, 0, "the __TEXT section must be first");
                }
            }
        }
    }

    assert_ne!(text_section.size, 0);

    if let Some(cmd) = commands.iter().find(|v| v.kind == LC_SYMTAB) {
        let mut s = Stream::new(&data[cmd.offset..], ByteOrder::LittleEndian);
        let symbols_offset: u32 = s.read();
        let number_of_symbols: u32 = s.read();
        let strings_offset: u32 = s.read();
        let strings_size: u32 = s.read();

        let strings = {
            let start = strings_offset as usize;
            let end = start + strings_size as usize;
            &data[start..end]
        };

        let symbols_data = &data[symbols_offset as usize..];
        return (
            parse_symbols(symbols_data, number_of_symbols, strings, text_section),
            text_section.size,
        );
    }

    (Vec::new(), 0)
}

#[derive(Clone, Copy, Debug)]
struct RawSymbol {
    string_index: u32,
    kind: u8,
    section: u8,
    address: u64,
}

fn parse_symbols(
    data: &[u8],
    count: u32,
    strings: &[u8],
    text_section: Section,
) -> Vec<SymbolData> {
    let mut raw_symbols = Vec::with_capacity(count as usize);
    let mut s = Stream::new(data, ByteOrder::LittleEndian);
    for _ in 0..count {
        let string_index: u32 = s.read();
        let kind: u8 = s.read();
        let section: u8 = s.read();
        s.skip::<u16>(); // description
        let value: u64 = s.read();

        if value == 0 {
            continue;
        }

        raw_symbols.push(RawSymbol {
            string_index,
            kind,
            section,
            address: value,
        });
    }

    // To find symbol sizes, we have to sort them by address.
    raw_symbols.sort_by_key(|v| v.address);

    // Add the __TEXT section end address, which will be used
    // to calculate the size of the last symbol.
    raw_symbols.push(RawSymbol {
        string_index: 0,
        kind: 0,
        section: 0,
        address: text_section.address + text_section.size,
    });

    let mut symbols = Vec::with_capacity(count as usize);
    for i in 0..raw_symbols.len() - 1 {
        let sym = &raw_symbols[i];

        if sym.string_index == 0 {
            continue;
        }

        const N_TYPE: u8   = 0x0E;
        const INDIRECT: u8 = 0xA;
        const SECTION: u8  = 0xE;

        let sub_type = sym.kind & N_TYPE;

        // Ignore indirect symbols.
        if sub_type & INDIRECT == 0 {
            continue;
        }

        // Ignore symbols without a section.
        if sub_type & SECTION == 0 {
            continue;
        }

        // Ignore symbols that aren't in the first section.
        // The first section is usually __TEXT,__text.
        if sym.section != 1 {
            continue;
        }

        // Mach-O format doesn't store the symbols size,
        // so we have to calculate it by subtracting an address of the next symbol
        // from the current.
        // Next symbol can have the same address as the current one,
        // so we have to find the one that has a different address.
        let next_sym = raw_symbols[i..].iter().skip_while(|s| s.address == sym.address).next();
        let size = match next_sym {
            Some(next) => next.address - sym.address,
            None => continue,
        };

        if let Some(s) = parse_null_string(strings, sym.string_index as usize) {
            symbols.push(SymbolData {
                name: crate::demangle::SymbolName::demangle(s),
                address: sym.address,
                size,
            });
        }
    }

    symbols
}
