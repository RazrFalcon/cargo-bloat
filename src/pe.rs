// See https://github.com/m4b/goblin/blob/master/src/pe/symbol.rs for details.

use crate::SymbolData;
use crate::parser::*;

const PE_POINTER_OFFSET: usize = 0x3c;
const COFF_SYMBOL_SIZE: usize = 18;
const IMAGE_SYM_CLASS_EXTERNAL: u8 = 2;
const IMAGE_SYM_DTYPE_SHIFT: usize = 4;
const IMAGE_SYM_DTYPE_FUNCTION: u16 = 2;
const SIZEOF_PE_MAGIC: usize = 4;
const SIZEOF_COFF_HEADER: usize = 20;

pub fn parse(data: &[u8]) -> Vec<SymbolData> {
    let mut s = Stream::new_at(data, PE_POINTER_OFFSET, ByteOrder::LittleEndian);
    let pe_pointer = s.read::<u32>() as usize;

    let mut s = Stream::new_at(data, pe_pointer, ByteOrder::LittleEndian);
    s.read::<u32>(); // magic
    s.skip::<u16>(); // machine
    let number_of_sections: u16 = s.read();
    s.skip::<u32>(); // time_date_stamp
    let pointer_to_symbol_table: u32 = s.read();
    let number_of_symbols: u32 = s.read();
    let size_of_optional_header: u16 = s.read();
    s.skip::<u16>(); // characteristics

    let mut text_section_size = 0;
    {
        let sections_offset =
              pe_pointer
            + SIZEOF_PE_MAGIC
            + SIZEOF_COFF_HEADER
            + size_of_optional_header as usize;

        let mut s = Stream::new_at(data, sections_offset, ByteOrder::LittleEndian);
        for _ in 0..number_of_sections {
            let name = s.read_bytes(8);
            s.skip_len(8); // virtual_size + virtual_address
            let size_of_raw_data: u32 = s.read();
            s.skip_len(20); // other data

            let len = name.iter().position(|c| *c == 0).unwrap_or(8);
            if std::str::from_utf8(&name[0..len]) == Ok(".text") {
                text_section_size = size_of_raw_data;
                break;
            }
        }
    }

    let mut symbols = Vec::with_capacity(number_of_symbols as usize);

    // Add the .text section size, which will be used
    // to calculate the size of the last symbol.
    symbols.push(SymbolData {
        name: crate::demangle::SymbolName::demangle(".text"),
        size: text_section_size as u64,
    });

    let string_table_offset =
        pointer_to_symbol_table as usize
            + number_of_symbols as usize * COFF_SYMBOL_SIZE;

    let mut s = Stream::new_at(data, pointer_to_symbol_table as usize, ByteOrder::LittleEndian);
    let symbols_data = s.read_bytes(number_of_symbols as usize * COFF_SYMBOL_SIZE);

    let mut s = Stream::new(symbols_data, ByteOrder::LittleEndian);
    while !s.at_end() {
        let name = s.read_bytes(8);
        let value: u32 = s.read();
        let section_number: i16 = s.read();
        let kind: u16 = s.read();
        let storage_class: u8 = s.read();
        let number_of_aux_symbols: u8 = s.read();
        s.skip_len(number_of_aux_symbols as usize * COFF_SYMBOL_SIZE);

        if (kind >> IMAGE_SYM_DTYPE_SHIFT) != IMAGE_SYM_DTYPE_FUNCTION {
            continue;
        }

        if storage_class != IMAGE_SYM_CLASS_EXTERNAL {
            continue;
        }

        if section_number == 0 {
            continue;
        }

        let name = if !name.starts_with(&[0, 0, 0, 0]) {
            let len = name.iter().position(|c| *c == 0).unwrap_or(8);
            std::str::from_utf8(&name[0..len]).ok()
        } else {
            let mut s2 = Stream::new(&name[4..], ByteOrder::LittleEndian);
            let name_offset: u32 = s2.read();
            parse_null_string(data, string_table_offset + name_offset as usize)
        };

        if let Some(s) = name {
            symbols.push(SymbolData {
                name: crate::demangle::SymbolName::demangle(s),
                // At this point size represents a symbol address.
                size: value as u64,
            });
        }
    }

    // To find symbol sizes, we have to sort them by address.
    symbols.sort_by_key(|v| v.size);

    // PE format doesn't store the symbols size,
    // so we have to calculate it by subtracting an address of the next symbol
    // from the current.
    for i in 1..symbols.len() {
        let curr = symbols[i].size;
        let next_sym = symbols[i..].iter().skip_while(|s| s.size == curr).next();
        if let Some(next_sym) = next_sym {
            symbols[i].size = next_sym.size - curr;
        }
    }

    // Remove the last symbol, which is `.text` section size.
    symbols.pop();

    symbols
}
