use std::{
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use axum::extract::Path as AxumPath;
use axum::{
    Extension, Router,
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
};
use axum_server::Server;
use chrono::Local;
use db::Db;
use fern::Dispatch;
use log::LevelFilter;
use tokio::fs;
use users::device_login::DeviceLoginDatabase;

mod db;
mod timekeeping;
mod users;
mod utils;

pub const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    eprintln!("{name} has started v{version}...");

    setup_logger();

    log::debug!("Debug is enabled.");
    log::trace!("Trace is enabled.");

    let pool = db::init_db("sqlite://enzowebserver.db?mode=rwc").await;
    let db = Db::new().set_device_login(DeviceLoginDatabase::new(pool.clone()).await);

    tokio::spawn(http_server(db));

    tokio::select! {
        _ = shutdown_signal() => {
            log::info!("Shutdown signal received.");
        }
    }
    db::close_db(pool).await;
    eprintln!("{name} has ended...");
}

async fn http_server(db: Db) {
    let app = Router::new()
        .route("/external/timekeeping/css/{*file}", get(serve_css))
        .route(
            "/external/timekeeping",
            get(timekeeping::handle_timekeeping),
        )
        .layer(Extension(db));

    let addr = SocketAddr::from_str(DEFAULT_SERVER_ADDRESS).unwrap();

    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_css(AxumPath(file): AxumPath<String>) -> impl IntoResponse {
    let mut path = PathBuf::from("css");
    path.push(&file);

    match fs::read(path).await {
        Ok(contents) => Response::builder()
            .header("Content-Type", "text/css")
            .body(Body::from(contents))
            .unwrap(),
        Err(err) => {
            log::error!("Serving CSS file: {}", err);
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("CSS file not found"))
                .unwrap()
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut term = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut int = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = term.recv() => {
                log::info!("Received SIGTERM (systemd stop).");
            }
            _ = int.recv() => {
                log::info!("Received SIGINT (Ctrl+C).");
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, only Ctrl+C is supported directly
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        log::info!("Received Ctrl+C (Windows)");
    }
}

fn setup_logger() {
    let level_filter = match (Path::new("trace").exists(), Path::new("debug").exists()) {
        (true, true) | (true, false) => LevelFilter::Trace,
        (false, true) => LevelFilter::Debug,
        (false, false) => LevelFilter::Info, // Default level
    };

    if let Err(e) = Dispatch::new()
        .format(move |out, message, record| {
            let file = record.file().unwrap_or("unknown_file");
            let line = record.line().map_or(0, |l| l);

            match level_filter {
                LevelFilter::Off
                | LevelFilter::Error
                | LevelFilter::Warn
                | LevelFilter::Debug
                | LevelFilter::Trace => {
                    out.finish(format_args!(
                        "[{}][{}]: {} <{}:{}>",
                        Local::now().format("%b-%d-%Y %H:%M:%S.%f"),
                        record.level(),
                        message,
                        file,
                        line,
                    ));
                }
                LevelFilter::Info => {
                    out.finish(format_args!(
                        "[{}]: {} <{}:{}>",
                        record.level(),
                        message,
                        file,
                        line,
                    ));
                }
            }
        })
        .level(level_filter)
        .chain(io::stdout())
        .apply()
    {
        log::error!("Logger initialization failed: {:?}", e);
    }
}
