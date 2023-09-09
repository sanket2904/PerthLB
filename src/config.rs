use std::collections::HashMap;
use std::fs::File;
use std::io::{Error as IOError, Read};
use std::path::Path;
use std::sync::mpsc::Receiver;
use serde::Deserialize;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};



#[derive(Debug, Clone)]
pub struct Config {
    pub filename: String,
    pub base: BaseConfig,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct BaseConfig {
    pub frontends: HashMap<String, FrontendConfig>,
    pub backends: HashMap<String, BackendPool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FrontendConfig {
    pub listen_addr: String,
    pub backend: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct BackendPool {
    pub servers: HashMap<String, ServerConfig>,
    // pub health_check_interval: u64,
}
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ServerConfig {
    pub addr: String,
    pub weight: Option<u16>,
}

#[derive(Debug)]
pub enum ReadError {
    IOError(IOError),
    ParseError(Vec<toml::de::Error>),
    DecodeError(toml::de::Error),
}


impl From<IOError> for ReadError {
    fn from(e: IOError) -> ReadError {
        ReadError::IOError(e)
    }
}

impl From<toml::de::Error> for ReadError {
    fn from(e: toml::de::Error) -> ReadError {
        ReadError::DecodeError(e)
    }
}

impl From<Vec<toml::de::Error>> for ReadError {
    fn from(e: Vec<toml::de::Error>) -> ReadError {
        ReadError::ParseError(e)
    }
}

impl Config {
    pub fn new(file: &str) -> Result<Config, ReadError> {
        let mut contents = String::new();
        let mut file_buf = File::open(file).unwrap();
        file_buf.read_to_string(&mut contents).unwrap();
        let decode: BaseConfig = toml::from_str(&contents).unwrap();
        Ok(Config {
            filename: file.to_string(),
            base: decode,
        })
    }
    pub fn reload(&self) -> Result<BaseConfig,ReadError> { 
        let mut contents = String::new();
        let mut file_buf = File::open(&self.filename).unwrap();
        file_buf.read_to_string(&mut contents).unwrap();
        let decode: BaseConfig = toml::from_str(&contents).unwrap();
        Ok(decode)
    }
    //  create a channel to receive config change when config file changed
    pub fn subscribe(self) -> Receiver<BaseConfig> {
        let (config_tx, config_rx) = std::sync::mpsc::channel();
        let filename = self.filename.clone();
        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let watcher: Result<RecommendedWatcher, notify::Error> = Watcher::new(tx, notify::Config::default());
            match watcher {
                Ok(mut watching) => {
                    match watching.watch( Path::new(&filename.clone()) , RecursiveMode::NonRecursive) {
                        Ok(_) => {
                            loop {
                                match rx.recv() {
                                    Ok(event) => {
                                        println!("config file changed: {:?}", event);
                                        match self.clone().reload() {
                                            Ok(new_config) => {
                                               match config_tx.send(new_config) {
                                                    Ok(_) => println!("send new config success"),
                                                    Err(_) => println!("send new config failed"),
                                               }
                                            },
                                            Err(_) => println!("reload config failed"),
                                        }

                                    },
                                    Err(_) => println!("watch error") ,
                                }
                            }
                        },
                        Err(_) => println!("watch error1"),
                    }
                },
                Err(_) => println!("watch error2"),  
            }
        });
        config_rx
    }
}