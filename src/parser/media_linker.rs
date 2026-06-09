// //! This file is responsible for normalizing media links (anything using the
// ![]() syntax) to fetch them and pull and place them properly into the
// crowdanki manifest

// use std::{ffi::OsStr, path::PathBuf};

// use eyre::Result;

// /// Given a path, normalize it to be relative to the root of the media
// directory /// So if you had a full path like
// /Users/Me/Downloads/Deck/example.png -> ./example.png fn normalize_path(path:
// PathBuf) -> Result<PathBuf> { 	let media_directory: PathBuf =
// PathBuf::from("./media"); 	/// We want to move anything that's NOT a flash
// file 	fn is_flash_file(file: PathBuf) -> bool {
// 		if let Some(extension) = file.as_path().extension()
// 			&& extension == OsStr::new(".flash")
// 		{
// 			true
// 		} else {
// 			false
// 		}
// 	}

// 	if !is_flash_file(path) {
// 		// We want to be respective of hierarchy
// 		let final_name = path.basename();

// 		// Move the file to the location
// 		std::fs::rename(path, (media_directory).join(final_name))
// 		return final_name;
// 	}
// }
