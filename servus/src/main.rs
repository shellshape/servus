pub mod conf;
pub mod web;

use std::{collections::HashMap, io, net::IpAddr, path::Path};

use clap::Parser;
use config::{
    builder::DefaultState, Config, ConfigBuilder, ConfigError, Environment, File, FileFormat, Map,
    Source, Value, ValueKind,
};
use directories::ProjectDirs;
use env_logger::Env;
use log::{debug, info};

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file location.
    #[arg(short, long)]
    config: Option<String>,

    /// The serve address (for example: 0.0.0.0:80).
    #[arg(short, long)]
    address: Option<String>,

    /// The Logging level.
    #[arg(short, long, default_value = "info,actix_server=warn")]
    loglevel: String,

    /// Serve directories
    /// (<directory> or <servepath>:<directory>).
    #[arg(short, long)]
    serve: Vec<String>,
}

impl Source for Args {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> std::result::Result<Map<String, Value>, ConfigError> {
        let mut m: HashMap<String, Value> = Map::new();
        let uri: String = "command line arguments".into();

        if let Some(addr) = &self.address {
            m.insert(
                "address".into(),
                Value::new(Some(&uri), ValueKind::String(addr.clone())),
            );
        }

        let vals: Vec<Value> = self
            .serve
            .iter()
            .map(|v| {
                let v_split: Vec<&str> = v.splitn(2, ':').collect();
                let (servepath, directory): (&str, &str) = if v_split.len() == 1 {
                    ("", v_split[0])
                } else {
                    (v_split[0], v_split[1])
                };
                Value::new(
                    Some(&uri),
                    ValueKind::Table(Map::from([
                        (
                            "type".into(),
                            Value::new(Some(&uri), ValueKind::String("Local".into())),
                        ),
                        (
                            "directory".into(),
                            Value::new(Some(&uri), ValueKind::String(directory.into())),
                        ),
                        (
                            "servepath".into(),
                            Value::new(Some(&uri), ValueKind::String(servepath.into())),
                        ),
                    ])),
                )
            })
            .collect();

        if !vals.is_empty() {
            let vals = Value::new(Some(&uri), ValueKind::Array(vals));
            m.insert("stores".into(), vals);
        }

        Ok(m)
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or(&args.loglevel))
        .try_init()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut cfg = Config::builder();

    cfg = add_source_files_for_dir(cfg, "config");

    if let Some(dirs) = ProjectDirs::from("de", "zekro", "servus") {
        let pth = dirs
            .config_dir()
            .to_str()
            .expect("could not transform path to string");
        cfg = add_source_files_for_dir(cfg, pth);
    }

    if let Some(config_dir) = &args.config {
        let p = Path::new(&config_dir);
        cfg = cfg.add_source(File::from(p).required(true));
    }

    let cfg = cfg
        .add_source(Environment::with_prefix("SERVUS").separator("_"))
        .add_source(args)
        .build()
        .expect("Failed to build config from sources")
        .try_deserialize::<conf::Config>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    cfg.validate()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    debug!("{:#?}", cfg);

    let bind_addr = cfg.address.unwrap_or_else(|| "0.0.0.0:80".into());
    let (ip_addr, port) = unwrap_address(&bind_addr)?;

    info!(
        "Bound to address {}:{}",
        if ip_addr.is_unspecified() {
            local_ip_address::local_ip().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        } else {
            ip_addr
        },
        port
    );

    cfg.stores
        .iter()
        .for_each(|s| info!("{}: {} -> /{}", s.name(), s.directory(), s.servepath()));

    web::run((ip_addr, port), cfg.stores).await
}

fn add_source_files_for_dir(
    cfg: ConfigBuilder<DefaultState>,
    name: &str,
) -> ConfigBuilder<DefaultState> {
    cfg.add_source(File::new(&format!("{name}.yaml"), FileFormat::Yaml).required(false))
        .add_source(File::new(&format!("{name}.yml"), FileFormat::Yaml).required(false))
        .add_source(File::new(&format!("{name}.toml"), FileFormat::Toml).required(false))
        .add_source(File::new(&format!("{name}.json"), FileFormat::Json5).required(false))
}

fn unwrap_address(addr: &str) -> io::Result<(IpAddr, u16)> {
    let (mut ip_addr, port) = if let Some(last_index) = addr.rfind(':') {
        addr.split_at(last_index)
    } else {
        (addr, ":80")
    };

    if ip_addr.is_empty() {
        ip_addr = "0.0.0.0"
    }

    if ip_addr == "localhost" {
        ip_addr = "127.0.0.1"
    }

    let port: u16 = port[1..]
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let ip_addr: IpAddr = ip_addr
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    Ok((ip_addr, port))
}
