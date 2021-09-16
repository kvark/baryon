use naga::{front::wgsl, valid::Validator};
use std::{fs, path::PathBuf};

/// Runs through all pass shaders and ensures they are valid WGSL.
#[test]
fn parse_wgsl() {
    let read_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("pass")
        .read_dir()
        .unwrap();

    for file_entry in read_dir {
        let shader = match file_entry {
            Ok(entry) => match entry.path().extension() {
                Some(ostr) if &*ostr == "wgsl" => {
                    println!("Validating {:?}", entry.path());
                    fs::read_to_string(entry.path()).unwrap_or_default()
                }
                _ => continue,
            },
            Err(e) => {
                log::warn!("Skipping file: {:?}", e);
                continue;
            }
        };

        let module = wgsl::parse_str(&shader).unwrap();
        //TODO: re-use the validator
        Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}
