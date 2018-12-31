use super::{HttpRequest, WebResult, PAGE_TITLE};
use actix_web::{HttpResponse, Responder};
use std::path;

fn sanitize_html_string<'a>(string: &'a str) -> std::borrow::Cow<'a, str> {
    [
        ('&', "&amp;"),
        ('"', "&quot;"),
        ('\'', "&#x27;"),
        ('<', "&lt;"),
        ('>', "&gt;"),
    ]
    .iter()
    .fold(
        std::borrow::Cow::Borrowed(string),
        |s, &(character, entity)| {
            if s.contains(character) {
                s.replace(character, entity).into()
            } else {
                s
            }
        },
    )
}

struct EntrySubfolder {
    given_path: path::PathBuf,
}

struct EntryFile {
    file_path: path::PathBuf,
    file_size: u64,
}

fn directory_listing(
    given_path: &path::Path,
    actual_path: &path::Path,
) -> Result<(Vec<EntrySubfolder>, Vec<EntryFile>), actix_web::error::Error> {
    let mut subfolders = Vec::new();
    let mut files = Vec::new();
    for entry in actual_path.read_dir()? {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if let Some(entry_file_name) = entry_path.file_name() {
                let entry_given_path = given_path.join(entry_file_name);
                if entry_path.is_dir() {
                    subfolders.push(EntrySubfolder {
                        given_path: entry_given_path,
                    })
                } else {
                    if entry_path.is_file() {
                        files.push(EntryFile {
                            file_path: entry_given_path,
                            file_size: entry.metadata().map(|metadata| metadata.len()).unwrap_or(0),
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

pub fn do_browse_directory(
    req: &HttpRequest,
    given_path: &path::Path,
    actual_path: &path::Path,
) -> WebResult {
    let (subfolders, files) = directory_listing(given_path, actual_path)?;
    let current_path: String = given_path
        .ancestors()
        .filter_map(|partial_path| {
            if let Some(file_name) = partial_path.file_name() {
                req.url_for("browse", &[partial_path.to_string_lossy()])
                    .ok()
                    .map(|path_url| (file_name.to_string_lossy(), path_url))
            } else {
                None
            }
        })
        .chain(
            req.url_for("browse", &[""])
                .ok()
                .map(|path_url| ("/".into(), path_url)),
        )
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .fold(
            (String::new(), " "),
            |(r, mut sep), (file_name, url_link)| {
                let new = format!(
                    r#"<a href="{1}">{0}</a>{2}"#,
                    sanitize_html_string(&file_name),
                    url_link,
                    sep,
                );
                sep = " / ";
                (r + &new, sep)
            },
        )
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
            files_table.push_str(&format!(
                r#"<tr class="folder_listing_header"><td>{0}<br />{1}</td><td></td></tr>
                "#,
                folders_line, files_line,
            ))
        }
        // subfolders
        for subfolder in subfolders {
            let path: &path::Path = &subfolder.given_path;
            let file_name_opt = path.file_name().map(|n| n.to_string_lossy());
            files_table.push_str(&format!(
                r#"<tr><td><a href="{1}">{0}</a></td><td>--</td></tr>
                "#,
                file_name_opt
                    .as_ref()
                    .map(|s| sanitize_html_string(&s))
                    .unwrap_or("".into()),
                req.url_for("browse", &[path.to_string_lossy()])?,
            ));
        }
        // files
        for file in files {
            let path: &path::Path = &file.file_path;
            let file_name_opt = path.file_name().map(|n| n.to_string_lossy());
            files_table.push_str(&format!(
                r#"<tr><td><a href="{1}">{0}</a></td><td>{2}</td></tr>
                "#,
                file_name_opt
                    .as_ref()
                    .map(|s| sanitize_html_string(&s))
                    .unwrap_or("".into()),
                req.url_for("browse", &[path.to_string_lossy()])?,
                file.file_size,
            ));
        }
        files_table
    };
    HttpResponse::Ok()
        .body(format!(
            r#"
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
            PAGE_TITLE, files_table, current_path,
        ))
        .respond_to(&req)
}
