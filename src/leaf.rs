use crate::{steps::Steps, Operation};
use anyhow::{bail, Context, Result};
use ignore::WalkBuilder;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};

#[derive(Debug, PartialEq, Deserialize)]
pub struct Leaf {
    #[serde(flatten, default)]
    custom: HashMap<String, Steps>,

    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Leaf {
    pub fn all_leaves() -> Result<Vec<Leaf>> {
        let mut leaves = Vec::new();
        for leaf in find_fern_files() {
            leaves.push(Leaf::from_file(leaf)?);
        }
        Ok(leaves)
    }

    pub fn here() -> Result<Leaf> {
        Leaf::from_file(PathBuf::from("./fern.yaml"))
    }

    fn from_yaml<R: std::io::Read>(source: R) -> Result<Self> {
        serde_yaml::from_reader(source).context("There was an error when reading the file")
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.path.clone().unwrap()
    }

    fn from_file(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            bail!("Did not find a fern.yaml file in here")
        }

        let file = File::open(path.clone()).unwrap();

        let mut config = Leaf::from_yaml(file)?;

        config.path = Some(path);

        Ok(config)
    }

    pub(crate) fn operations(&self) -> HashSet<String> {
        let mut operations = HashSet::new();
        for (op, steps) in &self.custom {
            if steps.any() {
                operations.insert(op.to_string());
            }
        }

        operations
    }

    pub(crate) fn run(self, op: &Operation) -> Result<()> {
        let steps = self
            .custom
            .get(&op.0)
            .cloned()
            .unwrap_or_else(Steps::default);

        let file_path = self.path.unwrap();
        let cwd = file_path.parent().unwrap();
        steps.run_all(&cwd)
    }
}

fn find_fern_files() -> Vec<PathBuf> {
    let mut fern_leaves = Vec::new();
    for result in WalkBuilder::new("./").build() {
        let entry = result.unwrap();
        if entry.metadata().unwrap().is_dir() {
            continue;
        }

        if entry.path().file_name().unwrap() != "fern.yaml" {
            continue;
        }

        fern_leaves.push(entry.into_path());
    }

    fern_leaves
}

#[cfg(test)]
mod tests {
    use crate::Leaf;

    #[test]
    fn it_parses_a_correct_yaml_file() {
        let yaml = r#"
       fmt: Something
       build: Fancy
       "#;

        let folder = Leaf::from_yaml(yaml.as_bytes()).unwrap();

        let possible_operations = folder.operations();

        assert!(possible_operations.contains("fmt"));
        assert!(possible_operations.contains("build"));
    }

    #[test]
    fn it_reports_adequate_errors() {
        let yaml = r#"fmt: Something
        has no value:
       "#;

        let error = Leaf::from_yaml(yaml.as_bytes()).unwrap_err().to_string();

        assert_eq!("There was an error when reading the file", error)
    }

    #[test]
    fn it_reports_errors_for_known_keys() {
        let yaml = "fmt: 12";

        let error = Leaf::from_yaml(yaml.as_bytes()).unwrap_err().to_string();

        assert_eq!("There was an error when reading the file", error)
    }
}
