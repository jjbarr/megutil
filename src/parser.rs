use std::io::{SeekFrom, prelude::*};
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::fmt;
use std::error;
use byteorder::{ReadBytesExt, LittleEndian};

const ENCRYPTED: u32 = 0x8FFFFFFF;
const NOCRYPT: u32 = 0xFFFFFFFF;
const MAGIC: u32 = 0x3F7D70A4;
const FILECRYPT: u16 = 0x01;

#[derive(Debug)]
pub enum MegFileError {
	IOError(std::io::Error),
	BadlyFormed,
	Encrypted
}

impl fmt::Display for MegFileError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self {
			Self::IOError(e) => format!("{}", e),
			Self::BadlyFormed => "File is not a valid archive.".to_string(),
			Self::Encrypted => "Archive is encrypted".to_string()
		})
	}
}

impl error::Error for MegFileError {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Self::IOError(e) => Some(e as &dyn error::Error),
			_ => None
		}
	}
}

impl From<std::io::Error> for MegFileError {
	fn from(e: std::io::Error) -> Self {
		Self::IOError(e)
	}
}

#[derive(Debug)]
pub enum ExtractionError {
	IOError(std::io::Error),
	NoSuchFile(String),
}

impl fmt::Display for ExtractionError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self {
			Self::IOError(e) => format!("{}", e),
			Self::NoSuchFile(s) => format!("file {} not present in archive", s),
		})
	}
}

impl error::Error for ExtractionError {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Self::IOError(e) => Some(e as &dyn error::Error),
			_ => None
		}
	}
}

impl From<std::io::Error> for ExtractionError {
	fn from(e: std::io::Error) -> Self {
		Self::IOError(e)
	}
}

pub struct MegFile<T: BufRead + Seek> {
	files: HashMap<String, ArchiveFile>,
	source: T
}

pub struct ArchiveFile {
	size: u32,
	start: u32,
}

fn build_name_tab<T>(src: &mut T, numfiles: u32, name_tab_size: u32)
                     -> Result<Vec<String>, MegFileError>
where T: BufRead + Seek {
	let mut cbytes = 0;
	let mut nametab = vec![];
	for _ in 0..numfiles {
		let name_size = src.read_u16::<LittleEndian>()?;
		let name =
			match String::from_utf8(src.bytes().take(name_size as usize)
			                        .collect::<Result<Vec<u8>,_>>()?){
				Ok(s) => s,
				Err(_) => Err(MegFileError::BadlyFormed)?
			};
		cbytes += name_size as u32;
		if cbytes > name_tab_size {Err(MegFileError::BadlyFormed)?};
		nametab.push(name)
	}
	Ok(nametab)
}

fn build_file_tab<T>(src: &mut T, numfiles: u32, name_tab: Vec<String>)
                     -> Result<HashMap<String, ArchiveFile>, MegFileError>
where T: BufRead + Seek {
	let mut file_tab = HashMap::new();
	for _ in 0..numfiles {
		let flags = src.read_u16::<LittleEndian>()?;
		if (flags & FILECRYPT) != 0 {Err(MegFileError::Encrypted)?};
		let _crc = src.read_u32::<LittleEndian>()?;
		let _idx = src.read_u32::<LittleEndian>()?;
		let size = src.read_u32::<LittleEndian>()?;
		let start = src.read_u32::<LittleEndian>()?;
		let nameidx = src.read_u16::<LittleEndian>()?;
		let name = name_tab[nameidx as usize].clone();
		file_tab.insert(name, ArchiveFile{size, start});
	}
	Ok(file_tab)
}

impl<T: BufRead + Seek> MegFile<T> {
	pub fn new(mut src: T) -> Result<Self, MegFileError> {
		src.seek(SeekFrom::Start(0))?;
		let flags = src.read_u32::<LittleEndian>()?;
		let magic = src.read_u32::<LittleEndian>()?;
		if flags == ENCRYPTED {
			Err(MegFileError::Encrypted)?;
		} else if flags != NOCRYPT || magic != MAGIC  {
			Err(MegFileError::BadlyFormed)?;
		}
		let _data_offset = src.read_u32::<LittleEndian>()?;
		let numfiles = src.read_u32::<LittleEndian>()?;
		if numfiles != src.read_u32::<LittleEndian>()? {
			Err(MegFileError::BadlyFormed)?;
		}
		let name_tab_size = src.read_u32::<LittleEndian>()?;
		let name_tab = build_name_tab(&mut src, numfiles, name_tab_size)?;
		let file_tab = build_file_tab(&mut src, numfiles, name_tab)?;
		Ok(MegFile{files: file_tab, source: src})
	}
	pub fn filenames(&self) -> Keys<String, ArchiveFile> {
		self.files.keys()
	}
	
	pub fn extract<U: Write>(&mut self, filename: &str, dest: &mut U)
	                         -> Result<(), ExtractionError> {
		let file = self.files.get(filename)
			.ok_or(ExtractionError::NoSuchFile(filename.to_string()))?;
		self.source.seek(SeekFrom::Start(file.start as u64))?;
		let mut src = self.source.by_ref().take(file.size as u64);
		std::io::copy(&mut src, dest)?;
		Ok(())
	}
}
