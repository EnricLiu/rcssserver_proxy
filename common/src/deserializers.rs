use std::net::IpAddr;
use serde::{Deserialize, Deserializer};

pub fn deserialize_ipaddr<'de, D: Deserializer<'de>>(deserializer: D) -> Result<IpAddr, D::Error> {
    deserialize_ipaddr_opt(deserializer).map(|opt| opt.unwrap())
}

pub fn deserialize_ipaddr_opt<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<IpAddr>, D::Error> {
    let s = <String>::deserialize(deserializer)?;

    let addr = if s == "localhost" {
        Ok(IpAddr::from([127, 0, 0, 1]))
    } else {
        s.parse().map_err(serde::de::Error::custom)
    }?;

    Ok(Some(addr))
}