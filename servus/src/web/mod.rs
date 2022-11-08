mod storage_handler;

use self::storage_handler::StorageHandler;
use crate::conf::StoreType;
use actix_web::{web, App, HttpServer};
use std::{io, net, sync::Arc};

pub async fn run<A>(addr: A, sources: Vec<StoreType>) -> io::Result<()>
where
    A: net::ToSocketAddrs,
{
    let sources = Arc::new(sources);

    HttpServer::new(move || {
        let mut app = App::new();

        for s in sources.clone().iter() {
            app = app.route(
                &format!("{}/{{filename:.*}}", s.servepath()),
                web::get().to(StorageHandler::new(s.clone())),
            );
        }

        app
    })
    .bind(addr)?
    .run()
    .await
}
