use core::fmt;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Steps {
    pub values: Vec<String>,
}

impl Default for Steps {
    fn default() -> Self {
        Steps { values: Vec::new() }
    }
}

impl Steps {
    pub fn any(&self) -> bool {
        !self.values.is_empty()
    }

    pub(crate) fn run_all(self, cwd: &Path) -> Result<()> {
        for value in self.values {
            let ecode = Command::new("sh")
                .arg("-c")
                .arg(value.clone())
                .current_dir(cwd)
                .status()
                .with_context(|| format!("Did not find {}", value))?;
            if !ecode.success() {
                bail!(
                    "Failed to execute command '{}': exit code {}",
                    value,
                    ecode.code().unwrap_or(-1)
                )
            }
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for Steps {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OneOrManyVisitor {}

        impl<'de> Visitor<'de> for OneOrManyVisitor {
            type Value = Steps;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("either single string or sequence of strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Steps {
                    values: vec![value.to_owned()],
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();

                while let Ok(Some(val)) = seq.next_element::<String>() {
                    values.push(val)
                }

                Ok(Steps { values })
            }
        }

        deserializer.deserialize_any(OneOrManyVisitor {})
    }
}
