use crate::conf::StoreType;
use actix_files::NamedFile;
use actix_web::{error, Handler, HttpRequest, Result};
use std::{
    future::Future,
    io::ErrorKind,
    path::{Path, PathBuf},
    pin::Pin,
};

#[derive(Clone)]
pub(super) struct StorageHandler {
    store_type: StoreType,
}

impl StorageHandler {
    pub(super) fn new(store_type: StoreType) -> Self {
        Self { store_type }
    }
}

impl Handler<HttpRequest> for StorageHandler {
    type Output = Result<NamedFile>;
    type Future = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn call(&self, req: HttpRequest) -> Self::Future {
        let path: PathBuf = req.match_info().query("filename").parse().unwrap();
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
                    return Err(error::ErrorNotFound("not found"));
                }
                Ok(f)
            }),
        }
    }
}
