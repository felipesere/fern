use core::fmt;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::fs::File;

#[derive(Debug, Eq, PartialEq)]
struct Steps {
    values: Vec<String>,
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
                formatter.write_str("either \"auto\" or a port number")
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

#[derive(Debug, PartialEq, Deserialize)]
struct FolderConfig {
    fmt: Steps,
    build: Steps,
    test: Steps,
    check: Steps,
}

fn main() {
    let file = File::open("./fern.yaml").unwrap();

    let config: FolderConfig = serde_yaml::from_reader(file).unwrap();
    println!("{:#?}", config);
}
