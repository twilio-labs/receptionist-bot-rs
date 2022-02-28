use serde::Serialize;
use serde_json::to_writer_pretty;
use std::{fs::File, path::Path};

/// used during development to capture/inspect for mocking
pub fn write_serde_struct_to_file<P: AsRef<Path>>(path: P, obj: impl Serialize) {
    to_writer_pretty(&File::create(path).expect("unable to create file"), &obj)
        .expect("unable to write to file")
}
