use core::fmt;
use serde::de::{Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer};
use std::fs::File;

#[derive(Debug, Eq, PartialEq)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl<'de> Deserialize<'de> for OneOrMany {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        one_or_many(deserializer)
    }
}

fn one_or_many<'de, D>(deserializer: D) -> Result<OneOrMany, D::Error>
where
    D: Deserializer<'de>,
{
    struct OneOrManyVisitor {}

    impl<'de> Visitor<'de> for OneOrManyVisitor {
        type Value = OneOrMany;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("either \"auto\" or a port number")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
            Ok(OneOrMany::One(value.to_owned()))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut result = Vec::new();

            while let Ok(Some(val)) = seq.next_element::<String>() {
                result.push(val)
            }

            Ok(OneOrMany::Many(result))
        }
    }

    deserializer.deserialize_any(OneOrManyVisitor {})
}

#[derive(Debug, PartialEq, Deserialize)]
struct FolderConfig {
    fmt: OneOrMany,
    build: OneOrMany,
    test: OneOrMany,
    check: OneOrMany,
}

fn main() {
    let file = File::open("./fern.yaml").unwrap();

    let config: FolderConfig = serde_yaml::from_reader(file).unwrap();
    println!("Hello, world!");
    println!("{:?}", config);
}
