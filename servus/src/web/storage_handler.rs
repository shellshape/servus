use crate::conf::StoreType;
use actix_files::NamedFile;
use actix_web::{error, Either, Handler, HttpRequest, HttpResponse, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::{
    error::Error,
    fs::{self, DirEntry},
    future::Future,
    io::ErrorKind,
    path::{Path, PathBuf},
    pin::Pin,
};

#[derive(Serialize)]
struct BrowseData {
    dirname: String,
    entries: Vec<BrowseEntry>,
}

#[derive(Serialize)]
struct BrowseEntry {
    name: String,
    is_dir: bool,
}

impl TryFrom<DirEntry> for BrowseEntry {
    type Error = Box<dyn Error>;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let name = value.file_name();
        let meta = value.metadata()?;

        Ok(Self {
            name: name.to_string_lossy().into(),
            is_dir: meta.is_dir(),
        })
    }
}

#[derive(Clone)]
pub(super) struct StorageHandler {
    store_type: StoreType,
    handlebars: Handlebars<'static>,
}

impl StorageHandler {
    pub(super) fn new(store_type: StoreType) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("t_browse", include_str!("../../hbs/browse.hbs"))
            .expect("registering template browse failed");

        Self {
            store_type,
            handlebars,
        }
    }
}

impl Handler<HttpRequest> for StorageHandler {
    type Output = Result<Either<NamedFile, HttpResponse>>;
    type Future = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn call(&self, req: HttpRequest) -> Self::Future {
        let path: PathBuf = req.match_info().query("filename").parse().unwrap();
        let hb = self.handlebars.clone();

        match self.store_type.clone() {
            StoreType::Local(local) => Box::pin(async move {
                let f =
                    NamedFile::open(Path::new(&local.directory).join(path)).map_err(|e| match e
                        .kind()
                    {
                        ErrorKind::NotFound => error::ErrorNotFound("not found"),
                        _ => error::ErrorInternalServerError(e),
                    })?;

                let meta = f.file().metadata()?;
                if meta.is_dir() {
                    if !local.browse.unwrap_or_default() {
                        return Err(error::ErrorNotFound("not found"));
                    }

                    let contents =
                        fs::read_dir(f.path()).map_err(error::ErrorInternalServerError)?;

                    let entries: Result<_, _> = contents
                        .filter(|c| c.is_ok())
                        .map(|c| c.unwrap().try_into())
                        .collect();

                    let d = BrowseData {
                        dirname: f
                            .path()
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                        entries: entries?,
                    };

                    let r = hb
                        .render("t_browse", &d)
                        .map_err(error::ErrorInternalServerError)?;
                    return Ok(Either::Right(HttpResponse::Ok().body(r)));
                }
                Ok(Either::Left(f))
            }),
        }
    }
}
