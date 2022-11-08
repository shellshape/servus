use anyhow::bail;
use serde::Deserialize;
use std::ops::Deref;

macro_rules! gen_store_impl {
    ($s:ident) => {
        impl Store for $s {
            fn servepath(&self) -> &str {
                &self.servepath
            }

            fn name(&self) -> &'static str {
                stringify!($s)
            }
        }
    };
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum StoreType {
    Local(LocalStore),
}

impl Deref for StoreType {
    type Target = dyn Store;

    fn deref(&self) -> &Self::Target {
        match self {
            StoreType::Local(local) => local,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub address: Option<String>,
    pub stores: Vec<StoreType>,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.stores.iter().any(|s| {
            self.stores
                .iter()
                .filter(|s2| s2.servepath() == s.servepath())
                .count()
                > 1
        }) {
            bail!("Config 'stores' contains douplicate entries for 'servepath'");
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct LocalStore {
    pub servepath: String,
    pub directory: String,
}

gen_store_impl!(LocalStore);

pub trait Store: DisplayDirectory {
    fn name(&self) -> &'static str;
    fn servepath(&self) -> &str;
}

pub trait DisplayDirectory {
    fn directory(&self) -> &str;
}

impl DisplayDirectory for LocalStore {
    fn directory(&self) -> &str {
        &self.directory
    }
}
