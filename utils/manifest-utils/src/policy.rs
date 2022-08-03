use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Public};
use openssl::sign::Verifier;
use openssl::x509::store::{X509Store, X509StoreBuilder};
use openssl::x509::{X509StoreContext, X509};
use structopt::StructOpt;
use strum::{Display, EnumIter, EnumString, EnumVariantNames, IntoEnumIterator, VariantNames};

/// Policy configuration
#[derive(StructOpt, Clone, Debug, Default)]
pub struct PolicyConfig {
    /// Disable policy components
    #[structopt(
        long,
        env,
        parse(try_from_str),
        possible_values = Policy::VARIANTS,
        case_insensitive = true,
    )]
    pub policy_disable_component: Vec<Policy>,
    /// Whitelist property names (optionally filtered by value)
    // e.g.
    //  POLICY_TRUST_PROPERTY="prop1=1|2,prop2=3|4|5,prop3"
    //  POLICY_TRUST_PROPERTY=prop4
    #[structopt(
        long,
        env,
        parse(try_from_str = parse_property_match),
    )]
    pub policy_trust_property: Vec<(String, Match)>,
    #[structopt(skip)]
    pub trusted_keys: Option<Keystore>,
}

impl PolicyConfig {
    pub fn policy_set(&self) -> HashSet<Policy> {
        if self.policy_disable_component.contains(&Policy::All) {
            Default::default()
        } else {
            let mut components: HashSet<_> = Policy::iter().collect();
            components.retain(|c| self.policy_disable_component.contains(c).not());
            components
        }
    }

    #[inline]
    pub fn trusted_property_map(&self) -> HashMap<String, Match> {
        self.policy_trust_property.iter().cloned().collect()
    }
}

#[non_exhaustive]
#[derive(
    Clone, Copy, Debug, Hash, Eq, PartialEq, EnumIter, EnumVariantNames, EnumString, Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum Policy {
    All,
    ManifestSignatureValidation,
    ManifestCompliance,
    ManifestInetUrlCompliance,
    ManifestScriptCompliance,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq, Display)]
pub enum Match {
    All,
    Values(Vec<String>),
}

impl FromStr for Match {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // use '|' instead of ',' to support multi-value
        // environment variables
        let values: Vec<_> = s.split('|').map(|v| v.trim().to_string()).collect();
        Ok(if values.is_empty() {
            Match::All
        } else {
            Match::Values(values)
        })
    }
}

#[derive(Clone)]
pub struct Keystore {
    inner: Arc<RwLock<X509Store>>,
}

impl Default for Keystore {
    fn default() -> Self {
        let store = X509StoreBuilder::new().expect("SSL works").build();
        Self {
            inner: Arc::new(RwLock::new(store)),
        }
    }
}

impl Keystore {
    /// Reads DER or PEM certificates (or PEM certificate stacks) from `cert_dir` and creates new `X509Store`.
    pub fn load(cert_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut store = X509StoreBuilder::new()?;
        let cert_dir = std::fs::read_dir(cert_dir)?;
        for dir_entry in cert_dir {
            let cert = dir_entry?;
            let cert = cert.path();
            Self::load_cert(&mut store, cert)?;
        }
        let store = store.build();
        let inner = Arc::new(RwLock::new(store));
        Ok(Keystore { inner })
    }

    pub fn replace(&self, other: Keystore) {
        let store = {
            let mut inner = other.inner.write().unwrap();
            std::mem::replace(&mut (*inner), X509StoreBuilder::new().unwrap().build())
        };
        let mut inner = self.inner.write().unwrap();
        *inner = store;
    }

    /// Decodes byte64 `sig`, verifies `cert`and reads its pub key,
    /// prepares digest using `sig_alg`, verifies `data` using `sig` and pub key.
    pub fn verify_signature(
        &self,
        cert: impl AsRef<str>,
        sig: impl AsRef<str>,
        sig_alg: impl AsRef<str>,
        data: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let sig = crate::decode_data(sig)?;

        let pkey = self.verify_cert(cert)?;

        let msg_digest = MessageDigest::from_name(sig_alg.as_ref()).ok_or(anyhow::anyhow!(
            "Unknown signature algorithm: {}",
            sig_alg.as_ref()
        ))?;
        let mut verifier = Verifier::new(msg_digest, pkey.as_ref())?;
        if false == verifier.verify_oneshot(&sig, data.as_ref().as_bytes())? {
            return Err(anyhow::anyhow!("Invalid signature"));
        }
        Ok(())
    }

    fn load_cert(store: &mut X509StoreBuilder, cert: PathBuf) -> anyhow::Result<()> {
        let extension = Self::get_file_extension(&cert);
        let mut cert = File::open(cert)?;
        let mut cert_buffer = Vec::new();
        cert.read_to_end(&mut cert_buffer)?;
        match extension {
            Some(ref der) if der == "der" => {
                let cert = X509::from_der(&cert_buffer)?;
                store.add_cert(cert)?;
            }
            Some(ref pem) if pem == "pem" => {
                for cert in X509::stack_from_pem(&cert_buffer)? {
                    store.add_cert(cert)?;
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown certificate file extension")),
        };
        Ok(())
    }

    fn get_file_extension(path: &PathBuf) -> Option<String> {
        path.extension()
            .map(OsStr::to_ascii_lowercase)
            .and_then(|ex| ex.to_str().map(ToString::to_string))
    }

    fn verify_cert<S: AsRef<str>>(&self, cert: S) -> anyhow::Result<PKey<Public>> {
        let cert = crate::decode_data(cert)?;
        let cert = match X509::from_der(&cert) {
            Ok(cert) => cert,
            Err(_) => X509::from_pem(&cert)?,
        };
        let store = self
            .inner
            .read()
            .map_err(|err| anyhow::anyhow!("Err: {}", err.to_string()))?;
        let cert_chain = openssl::stack::Stack::new()?;
        let mut ctx = X509StoreContext::new()?;
        if false == ctx.init(&store, &cert, &cert_chain, |ctx| ctx.verify_cert())? {
            return Err(anyhow::anyhow!("Invalid certificate"));
        }
        Ok(cert.public_key()?)
    }
}

impl std::fmt::Debug for Keystore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Keystore")
    }
}

fn parse_property_match(input: &str) -> anyhow::Result<(String, Match)> {
    let mut split = input.splitn(2, '=');
    let property = split
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing property name"))?
        .to_string();
    let values = match split.next() {
        Some(s) => Match::from_str(s)?,
        None => Match::All,
    };
    Ok((property, values))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_config<S: AsRef<str>>(args: S) -> PolicyConfig {
        let arguments = shlex::split(args.as_ref()).expect("failed to parse arguments");
        PolicyConfig::from_iter(arguments)
    }

    #[test]
    fn policy_config() {
        let config = build_config("TEST");
        assert_eq!(config.policy_disable_component, Vec::default());
        assert_eq!(config.policy_trust_property, Vec::default());

        let config = build_config(
            "TEST \
            --policy-trust-property property",
        );
        assert_eq!(config.policy_disable_component, Vec::default());
        assert_eq!(
            config.policy_trust_property,
            vec![("property".to_string(), Match::All)]
        );

        let config = build_config(
            "TEST \
            --policy-disable-component all \
            --policy-trust-property property=value1|value2",
        );
        assert_eq!(config.policy_disable_component, vec![Policy::All]);
        assert_eq!(
            config.policy_trust_property,
            vec![(
                "property".to_string(),
                Match::Values(vec!["value1".to_string(), "value2".to_string()])
            )]
        );
    }
}
