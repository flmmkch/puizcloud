#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use rocket::response::NamedFile;
use std::path::{Path, PathBuf};

#[get("/<path..>")]
fn files_route(path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("my_files/").join(path)).ok()
}

fn main() {
    rocket::ignite().mount("/files/", routes![files_route]).launch();
}
