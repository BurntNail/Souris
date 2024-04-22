use std::collections::HashMap;
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ErrorKind};
use std::path::PathBuf;
use dirs::data_dir;
use sourisdb::store::Store;
use color_eyre::eyre::bail;
use sourisdb::values::Value;
use sourisdb::types::array::Array;

const DIR: &str = "daddydb/";

mod meta {
    pub const META_DB_FILE_NAME: &str = "meta.sdb";
    pub const DB_FILE_NAMES_KEY: &str = "existing_dbs";
}
use meta::*;

#[derive(Clone, Debug)]
pub struct State {
    base_location: PathBuf,
    meta: Store,
    dbs: HashMap<String, Store>,
}

impl State {
    pub async fn new () -> color_eyre::Result<Self> {
        #[tracing::instrument(level = "trace")]
        async fn get_store (location: PathBuf) -> color_eyre::Result<Store> {
            let mut file = match File::open(&location).await {
                Ok(f) => f,
                Err(e) => {
                    return if e.kind() == ErrorKind::NotFound {
                        trace!(?location, "File not found, getting empty store.");
                        return Ok(Store::default())
                    } else {
                        Err(e.into())
                    };
                }
            };

            let mut contents = vec![];
            let mut tmp = [0_u8; 128];

            loop {
                match file.read(&mut tmp).await? {
                    0 => break,
                    n => {
                        contents.extend(&tmp[0..n]);
                    }
                }
            }

            Ok(Store::deser(&contents)?)
        }

        #[tracing::instrument(level = "trace")]
        async fn get_internal_stores (meta: &Store, base: PathBuf) -> Option<HashMap<String, Store>> {
            let Some(Value::Array(Array(values))) = meta.get(&Value::String(DB_FILE_NAMES_KEY.into())) else {
                trace!("Unable to find existing databases - using none");
                return None;
            };

            let mut dbs = HashMap::new();

            for val in values {
                let Value::String(file_name) = val else {
                    trace!(?val, "Found non-string inside existing databases list");
                    continue;
                };

                match get_store(base.join(file_name)).await {
                    Ok(s) => {
                        dbs.insert(file_name.to_owned(), s);
                    },
                    Err(e) => {
                        trace!(?e, ?file_name, "Error getting database");
                        continue;
                    }
                }
            }


            Some(dbs)
        }

        let Some(base_location) = data_dir() else {
            bail!("Unable to find data directory");
        };
        let base_location = base_location.join(DIR);

        let mut meta = get_store(base_location.join(META_DB_FILE_NAME)).await?;
        let dbs = match get_internal_stores(&meta, base_location.clone()).await {
            Some(dbs) => dbs,
            None => {
                meta.insert(Value::String(DB_FILE_NAMES_KEY.into()), Value::Array(Array(vec![])));
                HashMap::default()
            }
        };

        Ok(Self {
            base_location,
            meta,
            dbs
        })
    }

    pub async fn save (&self) -> color_eyre::Result<()> {
        let metadata = self.meta.ser()?;
        trace!(bytes=?metadata.len(), "Writing metadata to file");

        let location = self.base_location.join(META_DB_FILE_NAME);
        let mut metadata_file = match File::create(&location).await {
            Ok(f) => f,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    //folder must not exist
                    trace!(?location, "Unable to find folder, creating");

                    create_dir_all(&self.base_location).await?;
                    File::create(&location).await?
                } else {
                    return Err(e.into());
                }
            }
        };

        metadata_file.write_all(&metadata).await?;
        Ok(())
    }
}
