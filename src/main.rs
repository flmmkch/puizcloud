#[macro_use]
extern crate serde_derive;
use actix_web::{fs::NamedFile, server, App, HttpResponse, Responder};
use std::path;
use toml;

mod config;
use self::config::Config;

mod state;
use self::state::PuizcloudState;

type HttpRequest = actix_web::HttpRequest<PuizcloudState>;

const PAGE_TITLE: &'static str = "Puizcloud";

type WebResult =
    std::result::Result<actix_web::dev::AsyncResult<actix_web::HttpResponse>, actix_web::Error>;

mod directory;
use self::directory::*;

fn file_not_found(file_path: &path::Path) -> actix_web::HttpResponse {
    HttpResponse::NotFound()
        .content_type("text/plain")
        .body(format!("HTTP Error 404 Not Found: {}", file_path.display()))
}

fn do_browse(req: &HttpRequest) -> WebResult {
    let given_path_string = req.match_info().get_decoded("tail").unwrap_or("".into());
    let given_path = path::Path::new(&given_path_string);
    if given_path.is_relative() {
        let actual_path = req.state().full_data_path().join(given_path);
        if actual_path.is_dir() {
            do_browse_directory(req, &given_path, &actual_path)
        } else {
            if actual_path.is_file() {
                NamedFile::open(&actual_path)
                    .expect("Failed to open named file")
                    .respond_to(&req)
                    .respond_to(&req)
            } else {
                // empty path: default folder
                if given_path.components().next().is_none() {
                    do_browse_directory(req, &path::Path::new(""), &actual_path)
                } else {
                    file_not_found(&given_path).respond_to(&req)
                }
            }
        }
    } else {
        file_not_found(&given_path).respond_to(&req)
    }
}

fn read_config() -> Config {
    use std::fs;
    use toml;
    let configuration_toml_file = path::Path::new("puizcloud.toml");
    let configuration_toml = match fs::read(&configuration_toml_file) {
        Ok(f) => toml::from_slice::<Config>(&f).map_err(|e| format!("{}", e)),
        Err(e) => Err(format!("{}", e)),
    };
    match configuration_toml {
        Ok(config) => config,
        Err(e) => panic!(
            "{} failed to be read: {}",
            configuration_toml_file.display(),
            e
        ),
    }
}

fn main() {
    let config = read_config();
    let puizcloud_state = PuizcloudState::new(config);
    let ip = puizcloud_state.config().ip.clone();
    let port = puizcloud_state.config().port.clone();
    println!("Listening on {}:{}", &ip, &port);
    println!(
        "To access the file server: http://{}:{}/browse/",
        &ip, &port
    );
    if puizcloud_state.full_data_path().is_dir() {
        println!(
            "Serving folder {}",
            puizcloud_state.full_data_path().display()
        );
    } else {
        panic!(
            "{} is not a directory",
            puizcloud_state.full_data_path().display()
        );
    }
    server::new(move || {
        App::with_state(puizcloud_state.clone()).resource("/browse/{tail:.*}", |r| {
            r.name("browse");
            r.get().f(do_browse);
        })
    })
    .bind(format!("{}:{}", ip, port))
    .unwrap()
    .run();
}
