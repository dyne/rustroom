#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::{
    collections::HashMap,
    ffi::CString,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Clap;
use json::{self, object};
use rand::{rngs::OsRng, RngCore};
use rocket::{config::Environment, Config, Data, State};
use zenroom::zencode_exec;

const BUFFER_LIMIT: u64 = 2 * 1024 * 1024; // 2 Mb

#[derive(Clap)]
#[clap(version = "0.1", author = "Danilo Spinella <oss@danyspin97.org>")]
struct Opts {
    #[clap(short, long, default_value = "127.0.0.1")]
    address: String,
    #[clap(short, long, default_value = "zencode", env = "CONTRACTS")]
    contracts_dir: String,
    #[clap(short, long, default_value = "9856")]
    port: u16,
}

fn get_random_name() -> String {
    let mut data = [0u8; 16];
    OsRng.fill_bytes(&mut data);
    data.iter().map(|byte| format!("{:x}", byte)).collect()
}

fn get_contracts(contract_dir: &str) -> Result<HashMap<String, (CString, CString)>> {
    fs::read_dir(contract_dir)
        .with_context(|| format!("unable to read directory {}", contract_dir))?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<PathBuf>, _>>()
        .context("unable to read contracts")?
        .iter()
        .filter(|path| {
            if let Some(ext) = Path::new(path).extension() {
                ext == "zen"
            } else {
                false
            }
        })
        .map(|path| -> Result<(String, (CString, CString))> {
            let contract = Path::new(path)
                .with_extension("")
                .file_name()
                .with_context(|| format!("unable to get fi.into()lename for file {:?}", path))?
                .to_str()
                .with_context(|| format!("unable to convert `{:?}` to String", path))?
                .to_string();
            Ok((
                contract,
                (
                    CString::new(fs::read(path)?)?,
                    CString::new({
                        let path = Path::new(path).with_extension("keys");
                        if path.exists() {
                            fs::read(&path)
                                .with_context(|| format!("unable to read file {:?}", path))?
                        } else {
                            Vec::new()
                        }
                    })?,
                ),
            ))
        })
        .collect::<Result<HashMap<String, (CString, CString)>>>()
}

#[get("/contracts/<contract>")]
fn contracts(
    contracts: State<HashMap<String, (CString, CString)>>,
    contract: String,
) -> Result<Option<String>> {
    contracts_post(contracts, contract, None)
}

#[post("/contracts/<contract>", format = "json", data = "<msg>")]
fn contracts_post(
    contracts: State<HashMap<String, (CString, CString)>>,
    contract: String,
    msg: Option<Data>,
) -> Result<Option<String>> {
    if let Some((contract, keys)) = &contracts.get(&contract) {
        let mut buf = String::new();
        if let Some(msg) = msg {
            msg.open()
                .take(BUFFER_LIMIT)
                .read_to_string(&mut buf)
                .context("unable to read from POST data")?;
        }
        let (res, success) = zencode_exec(
            contract.clone(),
            CString::new("")?,
            CString::new(buf)?,
            keys.clone(),
        );

        Ok(Some(json::stringify_pretty(
            object! {
                ret: if success { 1 } else { 0 },
                out: json::parse(&res.output).context("unable to parse json from zenroom output")?,
                err: res.logs,
            },
            4,
        )))
    } else {
        Ok(None)
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let mut data = [0u8; 32];
    OsRng.fill_bytes(&mut data);
    let secret_key = base64::encode(data);

    let config = Config::build(Environment::Production)
        .address(opts.address)
        .port(opts.port)
        .secret_key(secret_key)
        .finalize()?;

    rocket::custom(config)
        .mount("/", routes![contracts])
        .manage(get_contracts(&opts.contracts_dir)?)
        .launch();
    Ok(())
}
