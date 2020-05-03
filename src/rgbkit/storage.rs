use std::path::PathBuf;
use std::{fs, io};

use lnpbp::bitcoin;
use lnpbp::bitcoin::hashes::hex::{FromHex, ToHex};
use lnpbp::rgb::prelude::*;

use super::file::*;
use super::InteroperableError;

pub trait Store {
    fn schema_ids(&self) -> Result<Vec<SchemaId>, InteroperableError>;
    fn schema(&self, id: SchemaId) -> Result<Schema, InteroperableError>;
    fn has_schema(&self, id: SchemaId) -> Result<bool, InteroperableError>;
    fn add_schema(&self, schema: &Schema) -> Result<bool, InteroperableError>;
    fn remove_schema(&self, id: SchemaId) -> Result<bool, InteroperableError>;

    fn contract_ids(&self) -> Result<Vec<ContractId>, InteroperableError>;
    fn genesis(&self, id: ContractId) -> Result<Genesis, InteroperableError>;
    fn has_genesis(&self, id: ContractId) -> Result<bool, InteroperableError>;
    fn add_genesis(&self, genesis: &Genesis) -> Result<bool, InteroperableError>;
    fn remove_genesis(&self, id: ContractId) -> Result<bool, InteroperableError>;
}

#[derive(Debug, Display, Error, From)]
#[display_from(Debug)]
pub enum DiskStorageError {
    #[derive_from]
    Io(io::Error),

    #[derive_from(bitcoin::hashes::Error)]
    HashName,

    #[derive_from]
    Encoding(lnpbp::strict_encoding::Error),

    #[derive_from(bitcoin::hashes::hex::Error)]
    BrokenHexFilenames,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display)]
#[display_from(Debug)]
pub struct DiskStorageConfig {
    pub data_dir: PathBuf,
}

impl DiskStorageConfig {
    pub const RGB_EXTENSION: &'static str = "rgb";

    #[inline]
    pub fn schemata_dir(&self) -> PathBuf {
        self.data_dir.join("schemata")
    }

    #[inline]
    pub fn geneses_dir(&self) -> PathBuf {
        self.data_dir.join("geneses")
    }

    #[inline]
    pub fn schema_filename(&self, schema_id: SchemaId) -> PathBuf {
        self.schemata_dir()
            .join(schema_id.to_hex())
            .with_extension(Self::RGB_EXTENSION)
    }

    #[inline]
    pub fn genesis_filename(&self, contract_id: ContractId) -> PathBuf {
        self.geneses_dir()
            .join(contract_id.to_hex())
            .with_extension(Self::RGB_EXTENSION)
    }

    #[inline]
    pub fn schema_names(&self) -> Result<Vec<String>, io::Error> {
        Ok(
            read_dir_filenames(self.schemata_dir(), Some(Self::RGB_EXTENSION))?
                .into_iter()
                .map(|name| String::from(name))
                .collect(),
        )
    }

    #[inline]
    pub fn genesis_names(&self) -> Result<Vec<String>, io::Error> {
        Ok(
            read_dir_filenames(self.geneses_dir(), Some(Self::RGB_EXTENSION))?
                .into_iter()
                .map(|name| String::from(name))
                .collect(),
        )
    }
}

/// Keeps all source/binary RGB contract data, stash etc
#[derive(Debug, Display)]
#[display_from(Debug)]
pub struct DiskStorage {
    config: DiskStorageConfig,
}

impl DiskStorage {
    pub fn new(config: DiskStorageConfig) -> Result<Self, DiskStorageError> {
        debug!("Instantiating RGB storage (disk storage) ...");

        let data_dir = config.data_dir.clone();
        if !data_dir.exists() {
            debug!(
                "RGB data directory '{:?}' is not found; creating one",
                data_dir
            );
            fs::create_dir_all(data_dir)?;
        }
        let schemata_dir = config.schemata_dir();
        if !schemata_dir.exists() {
            debug!(
                "RGB schemata directory '{:?}' is not found; creating one",
                schemata_dir
            );
            fs::create_dir_all(schemata_dir)?;
        }
        let geneses_dir = config.geneses_dir();
        if !geneses_dir.exists() {
            debug!(
                "RGB geneses data directory '{:?}' is not found; creating one",
                geneses_dir
            );
            fs::create_dir_all(geneses_dir)?;
        }

        Ok(Self { config })
    }
}

impl Store for DiskStorage {
    fn schema_ids(&self) -> Result<Vec<SchemaId>, InteroperableError> {
        self.config
            .schema_names()?
            .into_iter()
            .try_fold(vec![], |mut list, name| {
                list.push(SchemaId::from_hex(&name)?);
                Ok(list)
            })
    }

    #[inline]
    fn schema(&self, id: SchemaId) -> Result<Schema, InteroperableError> {
        Ok(Schema::read_file(self.config.schema_filename(id))?)
    }

    #[inline]
    fn has_schema(&self, id: SchemaId) -> Result<bool, InteroperableError> {
        Ok(self.config.schema_filename(id).as_path().exists())
    }

    fn add_schema(&self, schema: &Schema) -> Result<bool, InteroperableError> {
        let filename = self.config.schema_filename(schema.schema_id());
        let exists = filename.as_path().exists();
        schema.write_file(filename)?;
        Ok(exists)
    }

    fn remove_schema(&self, id: SchemaId) -> Result<bool, InteroperableError> {
        let filename = self.config.schema_filename(id);
        let existed = filename.as_path().exists();
        fs::remove_file(filename)?;
        Ok(existed)
    }

    fn contract_ids(&self) -> Result<Vec<ContractId>, InteroperableError> {
        self.config
            .genesis_names()?
            .into_iter()
            .try_fold(vec![], |mut list, name| {
                list.push(ContractId::from_hex(&name)?);
                Ok(list)
            })
    }

    #[inline]
    fn genesis(&self, id: ContractId) -> Result<Genesis, InteroperableError> {
        Ok(Genesis::read_file(self.config.genesis_filename(id))?)
    }

    #[inline]
    fn has_genesis(&self, id: ContractId) -> Result<bool, InteroperableError> {
        Ok(self.config.genesis_filename(id).as_path().exists())
    }

    fn add_genesis(&self, genesis: &Genesis) -> Result<bool, InteroperableError> {
        let filename = self.config.genesis_filename(genesis.contract_id());
        let exists = filename.as_path().exists();
        genesis.write_file(filename)?;
        Ok(exists)
    }

    #[inline]
    fn remove_genesis(&self, id: ContractId) -> Result<bool, InteroperableError> {
        let filename = self.config.genesis_filename(id);
        let existed = filename.as_path().exists();
        fs::remove_file(filename)?;
        Ok(existed)
    }
}