//! Cross-platform implementation of path swap.

#[macro_use]
extern crate log;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate lazy_static;

mod platform;

use std::{fs, io};
use std::path::Path;

/// Swaps the content of paths `a` and `b`.
pub fn swap<A, B>(a: A, b: B) -> io::Result<()> where A: AsRef<Path>, B: AsRef<Path> {
	platform::swap(a, b)
}

/// Nonatomic swap.
pub fn swap_nonatomic<A, B>(a: A, b: B) -> io::Result<()> where A: AsRef<Path>, B: AsRef<Path> {
	const TMP_SWAP_FILE: &'static str = "tmp.fs_swap";

	let a = a.as_ref();
	let b = b.as_ref();

	let tmp = a.parent()
		.or_else(|| b.parent())
		.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find a parent directory"))?
		.join(TMP_SWAP_FILE);

	warn!("Swapping {:?} and {:?} via {:?}", a, b, tmp);

	// cleanup
	match fs::metadata(&tmp) {
		Ok(ref meta) if meta.is_dir() => fs::remove_dir_all(&tmp)?,
		Ok(_) => fs::remove_file(&tmp)?,
		Err(ref err) if err.kind() == io::ErrorKind::NotFound => (),
		Err(err) => return Err(err),
	}

	// rename a to tmp
	// if it fails, the directories are unchanged
	fs::rename(a, &tmp)?;

	// Delete `a` (might still be there on Windows)
	match fs::metadata(&a) {
		Ok(ref meta) => {
			warn!("a metadata: {:?}", meta);
			if meta.is_dir() {
				fs::remove_dir_all(&a)?
			} else {
				fs::remove_file(&a)?
			}
		},
		Err(ref err) if err.kind() == io::ErrorKind::NotFound => (),
		Err(err) => return Err(err),
	}

	match fs::metadata(&b) {
		Ok(ref meta) => {
			warn!("b metadata: {:?}", meta);
		},
		_ => (),
	}

	match fs::rename(b, a) {
		Ok(_) => (),
		Err(err) => {
			// let's try to recover the previous state
			// if it fails, there is nothing we can do
			error!("swap_nonatomic failed b=>a: {:?}", err);
			return fs::rename(&tmp, a);
		},
	}

	// Delete `b` (might still be there on Windows)
	match fs::metadata(&b) {
		Ok(ref meta) if meta.is_dir() => fs::remove_dir_all(&b)?,
		Ok(_) => fs::remove_file(&b)?,
		Err(ref err) if err.kind() == io::ErrorKind::NotFound => (),
		Err(err) => return Err(err),
	}

	// rename tmp to b
	match fs::rename(&tmp, b) {
		Ok(_) => Ok(()),
		Err(err) => {
			// let's try to recover to previous state
			// if it fails, there is nothing we can do
			error!("swap_nonatomic failed tmp=>b: {:?}", err);
			fs::rename(a, b)?;
			fs::rename(&tmp, a)
		},
	}
}

#[cfg(test)]
mod tests {
	extern crate tempdir;
	use std::fs;
	use std::path::Path;
	use std::io::{Write, Read};
	use self::tempdir::TempDir;
	use super::{swap, swap_nonatomic};

	fn write_to_file<P: AsRef<Path>>(file: P, text: &str) {
		let mut file = fs::OpenOptions::new()
			.create(true)
			.write(true)
			.open(file)
			.unwrap();
		file.write_all(text.as_ref()).unwrap();
		file.flush().unwrap();
	}

	fn read_from_file<P: AsRef<Path>>(file: P) -> String {
		let mut buffer = String::new();
		let mut file = fs::OpenOptions::new()
			.read(true)
			.open(file)
			.unwrap();
		file.read_to_string(&mut buffer).unwrap();
		buffer
	}

	#[test]
	fn test_swap_files() {
		let dir = TempDir::new("").unwrap();
		let path_a = dir.path().join("file_a");
		let path_b = dir.path().join("file_b");
		write_to_file(&path_a, "foo");
		write_to_file(&path_b, "bar");
		swap(&path_a, &path_b).unwrap();
		let read_a = read_from_file(&path_a);
		let read_b = read_from_file(&path_b);
		assert_eq!("bar", read_a);
		assert_eq!("foo", read_b);
	}

	// atomic swaps of dirs are not supported on travis machines
	#[cfg(not(target_os = "macos"))]
	#[test]
	fn test_swap_dirs() {
		let dir_a = TempDir::new("a").unwrap();
		let dir_b = TempDir::new("b").unwrap();
		let path_a = dir_a.path().join("file");
		let path_b = dir_b.path().join("file");
		write_to_file(&path_a, "foo");
		write_to_file(&path_b, "bar");
		swap(&dir_a, &dir_b).unwrap();
		let read_a = read_from_file(&path_a);
		let read_b = read_from_file(&path_b);
		assert_eq!("bar", read_a);
		assert_eq!("foo", read_b);
	}

	#[test]
	fn test_swap_nonatomic_files() {
		let dir = TempDir::new("").unwrap();
		let path_a = dir.path().join("file_a");
		let path_b = dir.path().join("file_b");
		write_to_file(&path_a, "foo");
		write_to_file(&path_b, "bar");
		swap_nonatomic(&path_a, &path_b).unwrap();
		let read_a = read_from_file(&path_a);
		let read_b = read_from_file(&path_b);
		assert_eq!("bar", read_a);
		assert_eq!("foo", read_b);
	}

	// atomic swaps of dirs are not supported on travis machines
	#[test]
	fn test_swap_nonatomic_dirs() {
		let dir_a = TempDir::new("a").unwrap();
		let dir_b = TempDir::new("b").unwrap();
		let path_a = dir_a.path().join("file");
		let path_b = dir_b.path().join("file");
		write_to_file(&path_a, "foo");
		write_to_file(&path_b, "bar");
		swap_nonatomic(&dir_a, &dir_b).unwrap();
		let read_a = read_from_file(&path_a);
		let read_b = read_from_file(&path_b);
		assert_eq!("bar", read_a);
		assert_eq!("foo", read_b);
	}
}
