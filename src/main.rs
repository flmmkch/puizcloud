#[macro_use]
extern crate serde_derive;
use actix_web::{server, App, Responder, HttpResponse, fs::NamedFile};
use toml;
use std::{env, path};

mod config;
use self::config::Config;

#[derive(Debug, Clone)]
struct PuizcloudState {
    config: Config,
    full_data_path: path::PathBuf,
}

impl PuizcloudState {
    pub fn new(config: Config) -> PuizcloudState {
        let full_data_path = if config.data_path().is_absolute() {
            config.data_path().to_owned()
        }
        else {
            env::current_dir().expect("Unable to determine current directory")
                .join(config.data_path())
        };
        PuizcloudState {
            config,
            full_data_path,
        }
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn full_data_path(&self) -> &path::Path {
        &self.full_data_path
    }
}

type HttpRequest = actix_web::HttpRequest<PuizcloudState>;

struct EntrySubfolder {
    given_path: path::PathBuf,
}

struct EntryFile {
    file_path: path::PathBuf,
    file_size: u64,
}

fn directory_listing(given_path: &path::Path, actual_path: &path::Path) -> Result<(Vec<EntrySubfolder>, Vec<EntryFile>), actix_web::error::Error> {
    let mut subfolders = Vec::new();
    let mut files = Vec::new();
    for entry in actual_path.read_dir()? {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if let Some(entry_file_name) = entry_path.file_name() {
                if entry_path.is_dir() {
                    subfolders.push(EntrySubfolder {
                        given_path: given_path.join(entry_file_name),
                    })
                }
                else {
                    if entry_path.is_file() {
                        files.push(EntryFile {
                            file_path: entry_path,
                            file_size: entry.metadata().map(|metadata| metadata.len()).unwrap_or(0)
                        });
                    }
                }
            }
        }
    }
    subfolders.sort_unstable_by(|f1, f2| f1.given_path.cmp(&f2.given_path));
    files.sort_unstable_by(|f1, f2| f1.file_path.cmp(&f2.file_path));
    Ok((subfolders, files))
}

const PAGE_TITLE: &'static str = "Puizcloud";

type WebResult = std::result::Result<actix_web::dev::AsyncResult<actix_web::HttpResponse>, actix_web::Error>;

fn do_browse_directory(req: &HttpRequest, given_path: &path::Path, actual_path: &path::Path) -> WebResult {
    let (subfolders, files) = directory_listing(given_path, actual_path)?;
    let current_path: String = 
        given_path.components()
            .filter_map(|p| 
                if let path::Component::Normal(c) = p {
                    Some(c)
                }
                else {
                    None
                }
            )
            .fold((String::new(), path::PathBuf::new(), ""), |(r, p, mut sep), file_name| {
                let new_path = p.join(file_name);
                let result = if let Ok(url_link) = req.url_for("browse", &[new_path.to_string_lossy()]) {
                    let result =  r + sep + &format!(r#"<a href="{1}">{0}</a>"#,
                        file_name.to_string_lossy(),
                        url_link,
                        );
                    sep = " / ";
                    result
                }
                else {
                    r
                };
                (result, new_path, sep)
            })
            .0;
    let files_table = {
        let mut files_table = String::new();
        // header
        {
            let folders_line = match subfolders.len() {
                1 => "1 folder".to_owned(),
                n => format!("{} folders", n),
            };
            let files_line = match subfolders.len() {
                1 => "1 file".to_owned(),
                n => format!("{} files", n),
            };
            files_table.push_str(
                &format!(r#"<tr class="folder_listing_header"><td>{0}<br />{1}</td><td></td></tr>
                "#,
                folders_line,
                files_line,
                )
            )
        }
        // subfolders
        for subfolder in subfolders {
            let path: &path::Path = &subfolder.given_path;
            files_table.push_str(
                &format!(r#"<tr><td><a href="{1}">{0}</a></td><td>--</td></tr>
                "#,
                    path.file_name().map(|n| n.to_string_lossy()).unwrap_or("".into()),
                    req.url_for("browse", &[path.to_string_lossy()])?,
                    )
            );
        }
        // files
        for file in files {
            let path: &path::Path = &file.file_path;
            files_table.push_str(
                &format!(r#"<tr><td><a href="{1}">{0}</a></td><td>{2}</td></tr>
                "#,
                    path.file_name().map(|n| n.to_string_lossy()).unwrap_or("".into()),
                    req.url_for("browse", &[path.to_string_lossy()])?,
                    file.file_size,
                    )
            );
        }
        files_table
    };
    HttpResponse::Ok()
        .body(
            format!(r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <meta charset="utf-8">
                    <title>{0}</title>
                </head>
                <body>
                    <div id="current_path">{2}</div>
                    <table>{1}</table>
                </body>
                </html>
                "#,
                PAGE_TITLE,
                files_table,
                current_path,
            )
        )
        .respond_to(&req)
}

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
        }
        else {
            if actual_path.is_file() {
                NamedFile::open(&actual_path)
                    .expect("Failed to open named file")
                    .respond_to(&req)
                    .respond_to(&req)
            }
            else {
                // empty path: default folder
                if given_path.components().next().is_none() {
                    do_browse_directory(req, &path::Path::new(""), &actual_path)
                }
                else {
                    file_not_found(&given_path)
                        .respond_to(&req)
                }
            }
        }
    }
    else {
        file_not_found(&given_path)
            .respond_to(&req)
    }
}

fn read_config() -> Config {
    use std::fs;
    use toml;
    let configuration_toml_file = path::Path::new("./puizcloud.toml");
    fs::read(configuration_toml_file)
        .ok()
        .and_then(|v| toml::from_slice::<Config>(&v).ok())
        .unwrap_or_else(|| Config::default())
}

fn main() {
    let config = read_config();
    let puizcloud_state = PuizcloudState::new(config);
    let ip = puizcloud_state.config().ip.clone();
    let port = puizcloud_state.config().port.clone();
    println!("Listening on {}:{}", &ip, &port);
    println!("To access the file server: http://{}:{}/browse/", &ip, &port);
    println!("Serving folder {}", puizcloud_state.full_data_path().display());
    server::new(
        move || App::with_state(puizcloud_state.clone())
            .resource("/browse/{tail:.*}", |r| {
                r.name("browse");
                r.get().f(do_browse);
            })
        )
        .bind(format!("{}:{}", ip, port)).unwrap()
        .run();
}