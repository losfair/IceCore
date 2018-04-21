use std::collections::{BTreeSet, BTreeMap};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub applications: Vec<ApplicationConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub memory: AppMemoryConfig,
    #[serde(default)]
    pub optimize: bool,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(skip)]
    pub metadata: AppMetadata
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppMetadata {
    pub package_name: String,
    #[serde(default)]
    pub permissions: BTreeSet<AppPermission>,
    pub bin: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppMemoryConfig {
    pub min: usize,
    pub max: usize
}

impl Default for AppMemoryConfig {
    fn default() -> AppMemoryConfig {
        AppMemoryConfig {
            min: 64 * 65536,
            max: 256 * 65536
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum AppPermission {
    Timer,
    TcpListen(String /* address */),
    TcpListenAny,
    TcpConnect(String),
    TcpConnectAny,
    FileOpenReadOnlyAny,
    FileOpenReadWriteAny
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConfig {
    pub kind: ServiceKind
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServiceKind {
    Tcp,
    Http
}

fn read_and_parse_yaml_config<
    P: AsRef<Path> + ::std::fmt::Display,
    T
>(path: P) -> T
    where for<'de> T: ::serde::Deserialize<'de>
{
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(&path)
        .unwrap_or_else(|e| {
            panic!("Unable to open configuration file located at {}: {:?}", path, e)
        });

    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();

    ::serde_yaml::from_str(&text).unwrap_or_else(|e| {
        panic!("Unable to parse configuration: {:?}", e);
    })
}

impl Config {
    pub fn from_file(path: &str) -> Config {
        let mut config: Config = read_and_parse_yaml_config(path);

        for app in &mut config.applications {
            let app_root = app.path.clone();
            let metadata_path = Path::new(&app_root).join("config.yaml");

            app.metadata = read_and_parse_yaml_config(metadata_path.to_str().unwrap());
        }

        config
    }
}
