use crate::conf::StoreType;
use actix_files::NamedFile;
use actix_web::{error, Either, Handler, HttpRequest, HttpResponse, Result};
use handlebars::Handlebars;
use s3::{creds::Credentials, error::S3Error, serde_types::Object, Bucket, Region};
use serde::Serialize;
use std::{
    error::Error,
    fs::{self, DirEntry, File},
    future::Future,
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

impl From<&Object> for BrowseEntry {
    fn from(value: &Object) -> Self {
        BrowseEntry {
            name: value.key.clone(),
            is_dir: false,
        }
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
        let hb: Handlebars<'_> = self.handlebars.clone();

        match self.store_type.clone() {
            StoreType::Local(local) => Box::pin(async move {
                let path = Path::new(&local.directory).join(path);
                let f = File::open(&path)?;

                let meta = dbg!(f.metadata())?;
                if meta.is_dir() {
                    if !local.browse.unwrap_or_default() {
                        let index = path.join("index.html");
                        if index.exists() {
                            return Ok(Either::Right(
                                HttpResponse::Found()
                                    .append_header(("Location", "index.html"))
                                    .finish(),
                            ));
                        }
                        return Err(error::ErrorNotFound("not found"));
                    }

                    let contents = fs::read_dir(&path).map_err(error::ErrorInternalServerError)?;

                    let entries: Result<_, _> = contents
                        .filter(|c| c.is_ok())
                        .map(|c| c.unwrap().try_into())
                        .collect();

                    let d = BrowseData {
                        dirname: path
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

                let f = NamedFile::from_file(f, &path)?;
                Ok(Either::Left(f))
            }),

            StoreType::S3(s3) => Box::pin(async move {
                let creds =
                    Credentials::new(Some(&s3.accesskey), Some(&s3.secretkey), None, None, None)
                        .map_err(error::ErrorInternalServerError)?;

                let region = if let Some(endpoint) = s3.endpoint {
                    Region::Custom {
                        region: s3.region.unwrap_or_else(|| "us-east-1".into()),
                        endpoint,
                    }
                } else {
                    s3.region
                        .unwrap_or_else(|| Region::UsEast1.to_string())
                        .parse()
                        .map_err(error::ErrorInternalServerError)?
                };

                let bucket = Bucket::new(&s3.bucket, region, creds)
                    .map_err(error::ErrorInternalServerError)?
                    .with_path_style();

                if path.as_os_str().is_empty() {
                    if !s3.browse.unwrap_or_default() {
                        let head_res = bucket
                            .head_object(path.join("index.html").to_string_lossy())
                            .await;
                        return match head_res {
                            Ok(_) => Ok(Either::Right(
                                HttpResponse::Found()
                                    .append_header(("Location", "index.html"))
                                    .finish(),
                            )),
                            Err(S3Error::Http(404, _)) => Err(error::ErrorNotFound("not found")),
                            Err(err) => Err(error::ErrorInternalServerError(err)),
                        };
                    }

                    let entries: Vec<BrowseEntry> = bucket
                        .list("".into(), Some("/".into()))
                        .await
                        .map_err(error::ErrorInternalServerError)?
                        .first()
                        .ok_or_else(|| {
                            error::ErrorInternalServerError("no bucket found to list contents from")
                        })?
                        .contents
                        .iter()
                        .map(|v| v.into())
                        .collect();

                    let d = BrowseData {
                        dirname: bucket.name,
                        entries,
                    };

                    let r = hb
                        .render("t_browse", &d)
                        .map_err(error::ErrorInternalServerError)?;
                    return Ok(Either::Right(HttpResponse::Ok().body(r)));
                }

                let data = bucket
                    .get_object(path.to_str().unwrap_or_default())
                    .await
                    .map_err(error::ErrorNotFound)?;
                let data = data.bytes().to_vec();

                Ok(Either::Right(HttpResponse::Ok().body(data)))
            }),
        }
    }
}
