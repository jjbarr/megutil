//Copyright (C) Joshua Barrett 2020
//This code is licensed under the BSD 3-clause license. See LICENSE.txt for more
//information.
use clap::clap_app;
use std::process::exit;
use std::fs::{File, create_dir_all};
use std::io::prelude::*;
use std::io;
use std::path::Path;

mod parser;
use parser::MegFile;

//wrapper trait because we can't actually have multiple traits in a definition.
trait BufReadSeek : BufRead + Seek {}
impl<T> BufReadSeek for T where T: BufRead + Seek {}


enum Action {Extract, List}

fn open_archive(optpath: Option<&str>) ->
	Result<Box<dyn BufReadSeek>,io::Error> {
	if let Some(path) = optpath {
		let file = File::open(path)?;
		Ok(Box::new(io::BufReader::new(file)))
	} else {
		//We need to seek to handle files, and you can't seek on stdin.
		//So we use a Cursor.
		let mut bytes = vec![];
		io::stdin().lock().read_to_end(&mut bytes)?;
		Ok(Box::new(io::Cursor::new(bytes)))
	}
}

fn match_path<'a, T>(path: &str, archive: &'a MegFile<T>) -> Vec<&'a str>
where T: BufReadSeek{
	archive.filenames().filter(|file| {
		let searchpath = Path::new(path);
		let filepath = Path::new(file);
		searchpath.components().zip(filepath.components())
			.fold(true, |v, p| {
				v && (p.0 == p.1)
			})
	}).map(|s| s.as_str()).collect()
}

fn main() {
	let args = clap_app!(
		megutil =>
			(version: "0.1")
			(author: "Joshua Barrett")
			(about: "manipulate Petroglyph .meg archives")
			(@arg FILE: -f --file
			 +takes_value ".meg archive to use (default stdin)")
			(@arg verbose: -v --verbose)
			(@arg force: -F --force "extract files with rooted paths \
			                         (dangerous)")
			(@group action +required =>
			 (@arg extract: -x --extract
			  "extract all (or selected) files in archive")
			 (@arg list: -t --list
			  "list contents of archive")
			)
			(@arg FILES: ...)
	).get_matches();
	let verbose = args.is_present("verbose");
	let force = args.is_present("force");
	let archive_path = args.value_of("FILE");
	let action =
		if args.is_present("extract") {
			Action::Extract
		} else if args.is_present("list") {
			Action::List
		} else {
			eprintln!("{}", args.usage());
			exit(1);
		};
	let files: Vec<&str> = args.values_of("FILES").iter_mut()
		.flatten().collect();
	let mut archive = match MegFile::new(match open_archive(archive_path) {
		Ok(reader) => reader,
		Err(e) => {
			eprintln!("megutil: error opening archive: {}", e);
			exit(1);
		}
	}) {
		Ok(archive) => archive,
		Err(e) => {
			eprintln!("megutil: could not read archive: {}", e);
			exit(1);
		}
	};
	let opset : Vec<String>= if !files.is_empty() {
		files.iter().map(|&file| {
			let v = match_path(file, &archive);
			if v.is_empty() {
				eprintln!("megutil: {} not found in archive", file)
			}
			v
		}).flatten().map(|s| s.to_string()).collect()
	} else {
		archive.filenames().map(|s|{s.clone()}).collect()
	};
	match action {
		Action::Extract => {
			for file in opset {
				let fixedfile = file.replace("\\", "/"); //awful hack
				let path = Path::new(&fixedfile);
				if path.is_absolute() && !force {
					eprintln!("megutil: skipping {}: path is absolute\
					           (use -F to force extraction)", file);
					continue;
				}
				if let Some(parent) = path.parent() {
					match create_dir_all(parent) {
						Ok(_) => {},
						Err(e) => {
							eprintln!("megutil: could not create {}: {}",
							          parent.to_str().unwrap(),e);
							exit(1);
						}
					}
				}
				let mut out = match File::create(path) {
					Ok(out) => out,
					Err(e) => {
						eprintln!("megutil: could not create {}: {}",
						          path.to_str().unwrap(), e);
						exit(1);
					}
				};
				if let Err(e) = archive.extract(&file, &mut out) {
						eprintln!("megutil: error during extraction: {}", e);
						exit(1);
				}
				if verbose {println!("{}", path.to_str().unwrap())};
			}
		}
		Action::List => {
			for file in opset {
				println!{"{}", file};
			}
		}
	}
	exit(0);
}
