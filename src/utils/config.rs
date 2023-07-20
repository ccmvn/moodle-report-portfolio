use serde_derive::Deserialize;
use std::fs::File;
use std::io::Read;
use lazy_static::lazy_static;
use std::error::Error;
use std::path::{Path, PathBuf};

// Define the configuration file name
const CONFIG_FILE_NAME: &str = "config.toml";

// Function to generate the list of possible configuration file paths
fn generate_config_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("./"));
        paths.push(PathBuf::from("../"));
        paths.push(PathBuf::from("../../"));
    }

    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from(".\\"));
        paths.push(PathBuf::from("..\\"));
        paths.push(PathBuf::from("..\\..\\"));
    }
    
    paths.into_iter().map(|mut path| {
        path.push(CONFIG_FILE_NAME);
        path
    }).collect()
}

// Possible locations for the configuration file
lazy_static! {
    pub static ref FILES: Vec<PathBuf> = generate_config_file_paths();
}

// Function to check if the configuration file exists
pub fn config_exists(paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| Path::new(path).exists())
}

// Struct to hold the configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct GlobalConfig {
    pub account: AccountConfig,
    pub company: CompanyConfig,
    pub signature: SignatureConfig,
    pub website: WebsiteConfig,
    pub options: OptionsSettings,
}

// Struct to hold the account configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct AccountConfig {
    pub user_name: String,
    pub password: String,
}

// Struct to hold the company configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct CompanyConfig {
    pub educator_name: String,
    pub location: String,
}

// Struct to hold the signature configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct SignatureConfig {
    pub signature: String,
    pub font_name: String,
    pub font_size: u32,
}

// Struct to hold the website configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct WebsiteConfig {
    pub base_url: String,
}

// Struct to hold the configuration
#[derive(Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct OptionsSettings {
    pub test_mode: bool,
}

pub struct Config {
    config: GlobalConfig,
}

// Function to attempt opening the first available configuration file from a list of paths
fn open_first_available_file(paths: &[PathBuf]) -> std::io::Result<File> {
    let mut errors = Vec::new();

    for path in paths {
        match File::open(&path) {
            Ok(file) => return Ok(file),
            Err(e) => errors.push((path.clone(), e)),
        }
    }

    let error_message = errors
        .into_iter()
        .map(|(path, error)| format!("{}: {}", path.display(), error))
        .collect::<Vec<String>>()
        .join("\n");

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("No configuration file found:\n{}", error_message),
    ))
}

impl Config {
    // Constructor for the Config struct. It reads the configuration file and creates an instance of the struct
    pub fn new() -> Result<Config, Box<dyn Error>> {
        if !config_exists(&FILES) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Configuration file not found.",
            )));
        }

        let mut file = open_first_available_file(&FILES)?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;
        let config: GlobalConfig = toml::from_str(&contents)?;

        Ok(Config {
            config,
        })
    }

    // Getter for the user_name field
    pub fn get_user_name(&self) -> &str {
        &self.config.account.user_name
    }

    // Getter for the password field
    pub fn get_password(&self) -> &str {
        &self.config.account.password
    }

    // Getter for the educator_name field
    pub fn get_educator_name(&self) -> &str {
        &self.config.company.educator_name
    }

    // Getter for the location field
    pub fn get_location(&self) -> &str {
        &self.config.company.location
    }

    // Getter for the signature field
    pub fn get_signature(&self) -> &str {
        &self.config.signature.signature
    }

    // Getter for the font_name field
    pub fn get_font_name(&self) -> &str {
        &self.config.signature.font_name
    }

    // Getter for the font_size field
    pub fn get_font_size(&self) -> u32 {
        self.config.signature.font_size
    }
    
    // Getter for the base_url field
    pub fn get_base_url(&self) -> &str {
        &self.config.website.base_url
    }

    // Getter for the test_mode field
    pub fn get_test_mode(&self) -> bool {
        self.config.options.test_mode
    }
}

// Lazy_static is used to initialize the configuration only once and share it across the application
lazy_static! {
    static ref CONFIG: Config = Config::new().unwrap();
}
