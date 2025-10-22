use std::fs::File;
use std::io::Read;
use std::io::{Seek, SeekFrom};

use num_enum::TryFromPrimitive;

// TYPES
pub type ElfMagic = [u8;4];
pub type ElfVersion = u8;
pub type AbiVersion = u8;
pub type ExVersion = u32;
pub type ElfFlags = u32;


// ENUMS

#[derive(TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum ElfFormat {
    Arch32 = 1,
    Arch64 = 2
}

#[derive(TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum ElfEndianness {
    Little = 1,
    Big = 2
}

#[derive(TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum ElfAbi {
    SystemV = 0,
    HPUX = 1,
    NetBSD = 2,
    Linux = 3,
    GNUHurd = 4,
    Solaris = 5,
    AIXMonterey = 6,
    IRIX = 7,
    FreeBSD = 8,
    Tru64 = 9,
    NovellModesto = 10,
    OpenBSD = 11,
    OpenVMS = 12,
    NonStopKernel = 13,
    AROS = 14,
    FenixOS = 15,
    NuxiCloudABI = 16,
    StratusTechnologiesOpenVOS = 17
}

#[derive(TryFromPrimitive, PartialEq)]
#[repr(u16)]
pub enum ElfType {
    None = 0,
    Relocatable = 1,
    Executable = 2,
    Dynamic = 3,
    Core = 4,
    // compability
    LOOS = 0xFE00,
    HIOS = 0xFEFF,
    LOPROC = 0xFF00,
    HIPROC = 0xFFFF
}

#[derive(TryFromPrimitive, PartialEq)]
#[repr(u16)]
pub enum ElfMachine {
    None = 0x00,
    ATT = 0x01,
    SPARC = 0x02,
    X86 = 0x03,
    MotorolaM68k = 0x04,
    MotorolaM88k = 0x05,
    IntelMCU = 0x06,
    Intel80860 = 0x07,
    MIPS = 0x08,
    IBMSystem = 0x09,
    MIPSRS3000 = 0x0A,
    // some reserved
    HewlettPackardPARISC = 0x0F,
    Intel80960 = 0x13,
    PowerPC = 0x14,
    PowerPC64 = 0x15,
    S390 = 0x16,
    IBMSPU = 0x17,
    // some reserved
    NECV800 = 0x24,
    FujitsuFR20 = 0x25,
    TRWRH = 0x26,
    MotorolaRCE = 0x27,
    Arm = 0x28,
    DigitalAlpha = 0x29,
    SuperH = 0x2A,
    SPARCVersion9 = 0x2B,
    SiemensTriCore= 0x2C,
    ArgonautRISCCore = 0x2D,
    HitachiH8300 = 0x2E,
    HitachiH8300H = 0x2F,
    HitachiH8S = 0x30,
    HitachiH8500 = 0x31,
    IA64 = 0x32,
    StanfordMIPSX = 0x33,
    MotorolaColdFire = 0x34,
    MotorolaM68HC12 = 0x35,
    FujitsuMMAMultimediaAccelerator = 0x36,
    SiemensPCP = 0x37,
    SonynCPU = 0x38,
    DensoNDR1microprocessor = 0x39,
    MotorolaStar = 0x3A,
    ToyotaME16 = 0x3B,
    STMicroelectronicsST100 = 0x3C,
    AdvancedLogicCorpTinyJ = 0x3D,
    AMD64 = 0x3E,
    SonyDSP = 0x3F,
    DigitalEquipmentCorp10 = 0x40,
    DigitalEquipmentCorp11 = 0x41,
    SiemensFX66 = 0x42,
    STMicroelectronicsST9 = 0x43,
    STMicroelectronicsST7 = 0x44,
    MotorolaMC68HC16 = 0x45,
    MotorolaMC68HC11 = 0x46,
    MotorolaMC68HC08 = 0x47,
    MotorolaMC68HC05 = 0x48,
    SiliconGraphicsSVx = 0x49,
    STMicroelectronicsST19 = 0x4A,
    DigitalVAX = 0x4B,
    AxisCommunications = 0x4C,
    InfineonTechnologies = 0x4D,
    Element14 = 0x4E,
    LSILogic = 0x4F,
    TMS320C6000Family = 0x8C,
    MCSTElbruse2k = 0xAF,
    Arm64 = 0xB7,
    ZilogZ80 = 0xDC,
    RISCV = 0xF3,
    BerkeleyPacketFilter = 0xF7,
    WDC65C816 = 0x101,
    LoongArch  = 0x102
}

pub enum ElfAddress {
    Long(u32),
    Qword(u64)
}

// STRUCTS

pub struct ElfHeader {
    magic: ElfMagic,
    format: ElfFormat,
    endianness: ElfEndianness,
    version: ElfVersion,
    abi: ElfAbi,
    abi_version: AbiVersion,
    // 7 pad bytes
    etype: ElfType,
    emachine: ElfMachine,
    eversion: ExVersion,
    entry: ElfAddress,
    phoff: ElfAddress,
    shoff: ElfAddress,
    flags: ElfFlags,
    size: u16,
    phentry: u16, // entry size, not entry address
    phcount: u16,
    shentry: u16, // entry size, not entry address
    shcount: u16,
    index: u16
}

impl ElfHeader {
    pub fn new(&buffer: &[u8;16]) -> Self {     //takes in raw "magic" bytes (16bytes) from the file and converts them into a ElfHeader
        Self {
            magic: buffer[0..4].try_into().unwrap(),
            format: ElfFormat::try_from_primitive(buffer[4]).unwrap(),
            endianness: ElfEndianness::try_from_primitive(buffer[5]).unwrap(),
            version: buffer[6],
            abi: ElfAbi::try_from_primitive(buffer[7]).unwrap(),
            abi_version: buffer[8],
            etype: ElfType::None,
            emachine: ElfMachine::None,
            eversion: 0,
            entry: ElfAddress::Qword(0),
            phoff: ElfAddress::Qword(0),
            shoff: ElfAddress::Qword(0),
            flags: 0,
            size: 0,
            phentry: 0,
            phcount: 0,
            shentry: 0,
            shcount: 0,
            index: 0
       }
    }

    pub fn append(&mut self, &buffer: &[u8;48]) { // fills in the rest of the elf headers for 32 bit binaries
        match self.endianness {
            ElfEndianness::Little => {
                self.etype = ElfType::try_from_primitive(u16::from_le_bytes(buffer[0..2].try_into().unwrap())).unwrap();
                self.emachine = ElfMachine::try_from_primitive(u16::from_le_bytes(buffer[2..4].try_into().unwrap())).unwrap();
                self.eversion = u32::from_le_bytes(buffer[4..8].try_into().unwrap());
                let mut i = 32; //varible i is here so we dont have to repeat all of the lines that change the varibles but just some
                match self.format {
                    ElfFormat::Arch32 => {
                        self.entry = ElfAddress::Long(u32::from_le_bytes(buffer[8..12].try_into().unwrap()));
                        self.phoff = ElfAddress::Long(u32::from_le_bytes(buffer[12..16].try_into().unwrap()));
                        self.shoff = ElfAddress::Long(u32::from_le_bytes(buffer[16..20].try_into().unwrap()));
                        i -=12;
                    }
                    ElfFormat::Arch64 => {
                        self.entry = ElfAddress::Qword(u64::from_le_bytes(buffer[8..16].try_into().unwrap()));
                        self.phoff = ElfAddress::Qword(u64::from_le_bytes(buffer[16..24].try_into().unwrap()));
                        self.shoff = ElfAddress::Qword(u64::from_le_bytes(buffer[24..32].try_into().unwrap()));
                    }
                }
                self.flags = u32::from_le_bytes(buffer[i..i+4].try_into().unwrap());
                self.size = u16::from_le_bytes(buffer[i+4..i+6].try_into().unwrap());
                self.phentry = u16::from_le_bytes(buffer[i+6..i+8].try_into().unwrap());
                self.phcount = u16::from_le_bytes(buffer[i+8..i+10].try_into().unwrap());
                self.shentry = u16::from_le_bytes(buffer[i+10..i+12].try_into().unwrap());
                self.shcount = u16::from_le_bytes(buffer[i+12..i+14].try_into().unwrap());
                self.index = u16::from_le_bytes(buffer[i+14..i+16].try_into().unwrap());
            }
            ElfEndianness::Big => {
                self.etype = ElfType::try_from_primitive(u16::from_be_bytes(buffer[0..2].try_into().unwrap())).unwrap();
                self.emachine = ElfMachine::try_from_primitive(u16::from_be_bytes(buffer[2..4].try_into().unwrap())).unwrap();
                self.eversion = u32::from_be_bytes(buffer[4..8].try_into().unwrap());
                let mut i = 32; //varible i is here so we dont have to repeat all of the lines that change the varibles but just some
                match self.format {
                    ElfFormat::Arch32 => {
                        self.entry = ElfAddress::Long(u32::from_be_bytes(buffer[8..12].try_into().unwrap()));
                        self.phoff = ElfAddress::Long(u32::from_be_bytes(buffer[12..16].try_into().unwrap()));
                        self.shoff = ElfAddress::Long(u32::from_be_bytes(buffer[16..20].try_into().unwrap()));
                        i -=12;
                    }
                    ElfFormat::Arch64 => {
                        self.entry = ElfAddress::Qword(u64::from_be_bytes(buffer[8..16].try_into().unwrap()));
                        self.phoff = ElfAddress::Qword(u64::from_be_bytes(buffer[16..24].try_into().unwrap()));
                        self.shoff = ElfAddress::Qword(u64::from_be_bytes(buffer[24..32].try_into().unwrap()));
                    }
                }
                self.flags = u32::from_be_bytes(buffer[i..i+4].try_into().unwrap());
                self.size = u16::from_be_bytes(buffer[i+4..i+6].try_into().unwrap());
                self.phentry = u16::from_be_bytes(buffer[i+6..i+8].try_into().unwrap());
                self.phcount = u16::from_be_bytes(buffer[i+8..i+10].try_into().unwrap());
                self.shentry = u16::from_be_bytes(buffer[i+10..i+12].try_into().unwrap());
                self.shcount = u16::from_be_bytes(buffer[i+12..i+14].try_into().unwrap());
                self.index = u16::from_be_bytes(buffer[i+14..i+16].try_into().unwrap());
            }
        }
    }
    
    pub fn to_buffer(&self) -> Vec<u8> {
        vec![0]
    }
}



// FUNCTIONS

pub fn file_peek(file: &mut File) -> Vec<u8> {
    
}


fn get_magic(file: &mut File) -> ElfHeader {
    let mut buffer: [u8;16] = [0;16];
    file.read(&mut buffer).expect("File read error:");
    ElfHeader::new(&buffer)
}

pub fn elf_header(file: &mut File) -> ElfHeader{
    let mut header = get_magic(file);
    file.seek(SeekFrom::Start(16)).expect("IO Seek Error:");

    let mut buffer: [u8;48] = [0;48];
    file.read(&mut buffer).expect("File read error:");
    header.append(&buffer);

    header
}