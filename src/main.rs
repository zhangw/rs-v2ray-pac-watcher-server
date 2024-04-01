use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use env_logger::Env;
use log::{error, info};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

async fn poll_file_change(
    _data: web::Path<()>,
    rx_data: web::Data<Arc<Mutex<mpsc::Receiver<Result<notify::Event, notify::Error>>>>>,
) -> impl Responder {
    let rx = rx_data.lock().unwrap();

    loop {
        match rx.recv() {
            // Changed to `recv` to block until a message is received
            Ok(event) => {
                let mut paths: Vec<String> = Vec::new();
                event.unwrap().paths.iter().for_each(|path| {
                    info!(
                        "file changed, path:{:?}, is_dir:{:?}",
                        path.display(),
                        path.is_dir()
                    );
                    paths.push(path.display().to_string());
                });
                return HttpResponse::Ok().body(paths.join(","));
            }
            Err(e) => {
                error!("watch error: {:?}", e);
                continue;
            }
        }
    }
}

async fn health(_data: web::Path<()>) -> &'static str {
    "It works!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Usage: {} <file_path> <port>", args[0]);
        return Ok(());
    }
    let file_path = &args[1];
    let port = &args[2];
    let server_address = &format!("127.0.0.1:{}", port);

    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let config = Config::default()
        .with_poll_interval(Duration::from_secs(1))
        .with_compare_contents(true);
    let mut watcher = RecommendedWatcher::new(tx, config).unwrap();
    watcher
        .watch(file_path.as_ref(), RecursiveMode::Recursive)
        .unwrap();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(rx.clone()))
            .route("/poll/v2rayU/pac/proxy.js", web::get().to(poll_file_change))
            .route("/health", web::get().to(health))
    })
    .bind(server_address.clone())?
    .run()
    .await
}
