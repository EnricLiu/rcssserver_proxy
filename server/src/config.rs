use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

use common::error::ConfigError;
use common::deserializers::deserialize_ipaddr_opt;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct SocketConfig {
    pub url:    Option<String>,
    #[serde(deserialize_with = "deserialize_ipaddr_opt")]
    pub host:   Option<IpAddr>,
    pub port:   Option<u16>,
}

impl SocketConfig {
    fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        match (&self.url, self.host, self.port) {
            (Some(url), Some(host), Some(port)) => {
                let addr1 = url.parse::<SocketAddr>()
                    .map_err(|e| ConfigError::InvalidValue("url", e.to_string()))?;

                let addr2 = SocketAddr::new(host, port);
                (addr1 == addr2)
                    .then_some(addr1)
                    .ok_or(ConfigError::ConflictFields(vec!["url", "host", "port"]))

            },
            (Some(url), _, _) => {
                url.parse::<SocketAddr>()
                    .map_err(|e| ConfigError::InvalidValue("url", e.to_string()))
            },
            (None, Some(host), Some(port)) => {
                Ok(SocketAddr::new(host, port))
            },
            _ => {
                Err(ConfigError::MissingField("url"))
            }
        }
    }

}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializeConfig {
    pub rcsss:      SocketConfig,
    pub udp_proxy:  SocketConfig,
    pub ws:         SocketConfig,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub udp_rcsss_url:  SocketAddr,
    pub udp_proxy_url:  SocketAddr,
    pub ws_url:         SocketAddr,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let toml = std::fs::read_to_string(path).expect("Unable to read file");
        toml.parse()
    }
}

impl TryFrom<SerializeConfig> for Config {
    type Error = ConfigError;

    fn try_from(config: SerializeConfig) -> Result<Self, Self::Error> {
        let udp_rcsss_url = config.rcsss.socket_addr()?;
        println!("udp_rcsss_url: {}", udp_rcsss_url);
        let udp_proxy_url = config.udp_proxy.socket_addr()?;
        println!("udp_proxy_url: {}", udp_proxy_url);
        let ws_url = config.ws.socket_addr()?;
        println!("ws_url: {}", ws_url);

        Ok(Self {
            udp_rcsss_url,
            udp_proxy_url,
            ws_url,
        })
    }
}

impl FromStr for Config {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Config, ConfigError> {
        let cfg: SerializeConfig = toml::from_str(s)?;
        Ok(Config::try_from(cfg)?)
    }
}

mod test {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let cfg = r#"
        [rcsss]
        url = "127.0.0.1:8000"

        [udp_proxy]
        host = "localhost"
        port = 0

        [ws]
        url = "[A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267]:8002"
        host = "A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267"
        port = 8002
    "#;

        let cfg = Config::from_str(cfg).unwrap();
        assert_eq!(cfg.udp_rcsss_url, SocketAddr::from_str("127.0.0.1:8000").unwrap());
        assert_eq!(cfg.udp_proxy_url, SocketAddr::from_str("127.0.0.1:0").unwrap());
        assert_eq!(cfg.ws_url, SocketAddr::from_str("[A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267]:8002").unwrap());
    }

    #[test]
    fn test_parse_invalid() {
        // missing [rcsss.port]
        let cfg = r#"
        [rcsss]
        host = "localhost"

        [udp_proxy]
        host = "localhost"
        port = 0

        [ws]
        url = "[A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267]:8002"
        host = "A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267"
        port = 8002
    "#;

        assert!(Config::from_str(cfg).is_err());

        // missing [rcsss.host]
        let cfg = r#"
        [rcsss]
        port = 6666

        [udp_proxy]
        host = "localhost"
        port = 0

        [ws]
        url = "[A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267]:8002"
        host = "A4C6:8B7F:F064:323E:FE0C:936B:E3DD:D267"
        port = 8002
    "#;

        assert!(Config::from_str(cfg).is_err());

        // bad [udp_proxy.host]
        let cfg = r#"
        [rcsss]
        host = "localhost"
        port = 6666

        [udp_proxy]
        host = "localpost" # <-
        port = 0

        [ws]
        url = "127.0.0.1:8002"
        host = "localhost"
        port = 666
    "#;

        assert!(Config::from_str(cfg).is_err());

        // conflict ws
        let cfg = r#"
        [rcsss]
        host = "localhost"
        port = 6666

        [udp_proxy]
        host = "localhost"
        port = 0

        [ws]
        url = "127.0.0.1:8002"
        host = "localhost"
        port = 666
    "#;

        assert!(Config::from_str(cfg).is_err());
    }
}
