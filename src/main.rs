extern crate actix_web;
use actix_web::{server, App, Responder, HttpResponse, fs::NamedFile};
use std::path;

struct EntrySubfolder {
    subfolder_path: path::PathBuf,
}

struct EntryFile {
    file_path: path::PathBuf,
    file_size: u64,
}

fn directory_listing(dir_path: &path::Path) -> Result<(Vec<EntrySubfolder>, Vec<EntryFile>), actix_web::error::Error> {
    let mut subfolders = Vec::new();
    let mut files = Vec::new();
    for entry in dir_path.read_dir()? {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if let Some(_) = entry_path.file_name() {
                if entry_path.is_dir() {
                    subfolders.push(EntrySubfolder {
                        subfolder_path: entry_path,
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
    subfolders.sort_unstable_by(|f1, f2| f1.subfolder_path.cmp(&f2.subfolder_path));
    files.sort_unstable_by(|f1, f2| f1.file_path.cmp(&f2.file_path));
    Ok((subfolders, files))
}

const PAGE_TITLE: &'static str = "Puizcloud";

fn do_browse_directory(req: &actix_web::HttpRequest, given_path: &path::Path) -> std::result::Result<actix_web::dev::AsyncResult<actix_web::HttpResponse>, actix_web::Error> {
    let (subfolders, files) = directory_listing(&given_path)?;
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
            let path: &path::Path = &subfolder.subfolder_path;
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

fn do_browse(req: &actix_web::HttpRequest) -> std::result::Result<actix_web::dev::AsyncResult<actix_web::HttpResponse>, actix_web::Error> {
    let given_path = path::PathBuf::from(&req.match_info().query::<String>("tail")?);
    if given_path.is_relative() {
        if given_path.is_dir() {
            do_browse_directory(req, &given_path)
        }
        else {
            if given_path.is_file() {
                NamedFile::open(given_path)
                    .expect("Failed to open named file")
                    .respond_to(&req)
                    .respond_to(&req)
            }
            else {
                HttpResponse::NotFound()
                    .finish()
                    .respond_to(&req)
            }
        }
    }
    else {
        HttpResponse::Forbidden()
            .content_type("text/plain")
            .body("HTTP Error 404: Not Found")
            .respond_to(&req)
    }
}

fn main() {
    server::new(
        || App::new()
            .resource("/browse/{tail:.*}", |r| {
                r.name("browse");
                r.get().f(do_browse);
            })
        )
        .bind("127.0.0.1:8080").unwrap()
        .run();
}