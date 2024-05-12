use elf::ElfBytes;
use std::fs;
// Allow the list of function info to be sorted
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct FunctionInfo {
    func_name: String,
    file_name: String,
    line_number: u64,
    column_number: u64,
}

trait Reader: gimli::Reader<Offset = usize> {}
impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where Endian: gimli::Endianity {}
fn process_subprogram<R: Reader>(
    dwarf: &gimli::Dwarf<R>,
    header: &gimli::UnitHeader<R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Option<FunctionInfo> {
    let mut attrs = entry.attrs();
    let mut func_name: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut line_number: Option<u64> = None;
    let mut column_number: Option<u64> = None;

    while let Some(attr) = attrs.next().unwrap() {
        match attr.name() {
            gimli::DW_AT_name => {
                if let gimli::AttributeValue::DebugStrRef(d) = attr.value() {
                    func_name = Some(
                        dwarf
                            .debug_str
                            .get_str(d)
                            .unwrap()
                            .to_string()
                            .unwrap()
                            .to_string(),
                    );
                }
            }
            gimli::DW_AT_decl_file => {
                if let gimli::AttributeValue::FileIndex(val) = attr.value() {
                    let index = (val - 1) as usize;
                    // index 0 is the module file, so get the name of the module file
                    if index == 0 {
                        file_name = Some(
                            dwarf
                                .unit(header.clone())
                                .unwrap()
                                .name
                                .unwrap()
                                .to_string()
                                .unwrap()
                                .to_string(),
                        );
                    } else {
                        let unit = dwarf.unit(header.clone()).unwrap();
                        let line_program = unit.line_program.clone().unwrap();
                        let line_program_header = line_program.header();
                        let f = line_program_header.file_names().get(index).unwrap();
                        let dir_index = f.directory_index();
                        let dir = line_program_header.directory(dir_index).unwrap();
                        match dir {
                            gimli::AttributeValue::String(s) => {
                                let dir_str = s.to_string().unwrap();
                                let file_attr = dwarf.attr_string(&unit, f.path_name()).unwrap();
                                let file_str = file_attr.to_string().unwrap();
                                let path =
                                    std::path::Path::new(dir_str.as_ref()).join(file_str.as_ref());
                                file_name = Some(path.to_str().unwrap().to_string());
                            }
                            _otherwise => {}
                        }
                    }
                }
            }
            gimli::DW_AT_decl_line => {
                line_number = Some(attr.value().udata_value().unwrap());
            }
            gimli::DW_AT_decl_column => {
                column_number = Some(attr.value().udata_value().unwrap());
            }
            _otherwise => {}
        }
    }
    if let (Some(func), Some(file), Some(line), Some(col)) =
        (func_name, file_name, line_number, column_number)
    {
        // function has all the required fields
        return Some(FunctionInfo {
            func_name: func,
            file_name: file,
            line_number: line,
            column_number: col,
        });
    }
    return None;
}
// print for vim format
fn print_file_info(func_info: &FunctionInfo) {
    //let col = func_info.column_number;
    let line = func_info.line_number;
    let file = &func_info.file_name;
    let func = &func_info.func_name;
    println!("{}\t{}\t:{}", func, file, line);
}
// load_file_section want's to return an empty array when a section isn't found
static EMPTY_ARRAY: [u8; 0] = [0; 0];
// Create a function that gimli can call
// receives the requested section
// returns the requested section data
//
// The life time of the input file's data outlives this function
fn load_file_section<'input>(
    section: gimli::SectionId,
    file: &elf::ElfBytes<'input, elf::endian::AnyEndian>,
    endian: gimli::RunTimeEndian,
) -> gimli::Result<gimli::EndianSlice<'input, gimli::RunTimeEndian>> {
    // Get the requested section header
    let sec = file.section_header_by_name(section.name()).unwrap();
    if let Some(section_header) = sec {
        let section_data = file.section_data(&section_header).unwrap().0;
        // Return the found data
        Ok(gimli::EndianSlice::new(section_data, endian))
    } else {
        // No section header was found return the empty array
        return Ok(gimli::EndianSlice::new(&EMPTY_ARRAY, endian));
    }
}
fn main() {
    let path = std::env::args().nth(1).expect("no path given");

    let file_data = fs::read(path).expect("Should have been able to read the file");
    let slice = file_data.as_slice();
    // Get the Elf file
    let file = &ElfBytes::<'_, elf::endian::AnyEndian>::minimal_parse(slice).expect("Open test1");
    let endian;
    match file.ehdr.endianness {
        elf::endian::AnyEndian::Little => {
            endian = gimli::RunTimeEndian::Little;
        }
        elf::endian::AnyEndian::Big => {
            endian = gimli::RunTimeEndian::Big;
        }
    }
    // load will request each required setion from load_section
    let load_section =
        |id: gimli::SectionId| -> gimli::Result<gimli::EndianSlice<gimli::RunTimeEndian>> {
            load_file_section(id, file, endian)
        };
    let dwarf = gimli::Dwarf::load(load_section).unwrap();
    let mut iter = dwarf.units();
    let mut file_info_list: Vec<FunctionInfo> = Vec::new();
    while let Some(header) = iter.next().unwrap() {
        // Iterate over all of this compilation unit's entries
        let unit = dwarf.unit(header).unwrap();
        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs().unwrap() {
            match entry.tag() {
                gimli::DW_TAG_subprogram => {
                    // A function
                    let func_info = process_subprogram(&dwarf, &header, entry);
                    if let Some(f) = func_info {
                        file_info_list.push(f);
                    }
                }
                _otherwise => {}
            }
        }
    }


    // print the ctags header
    println!("!_TAG_FILE_FORMAT	2	/extended format; --format=1 will not append ;\" to lines/");
    println!("!_TAG_FILE_SORTED	1	/0=unsorted, 1=sorted, 2=foldcase/");
    // sort the ctags
    file_info_list.sort();
    // remove duplicates
    file_info_list.dedup();
    for f in file_info_list.iter() {
        print_file_info(f);
    }
}
