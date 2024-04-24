use color_eyre::eyre::bail;
use dirs::data_dir;
use sourisdb::{store::Store, types::array::Array, values::Value};
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, ErrorKind},
    sync::Mutex,
};

const DIR: &str = "souris/";

mod meta {
    pub const META_DB_FILE_NAME: &str = "meta.sdb";
    pub const DB_FILE_NAMES_KEY: &str = "existing_dbs";
}
use crate::error::SourisError;
use meta::*;
use sourisdb::utilities::cursor::Cursor;

#[derive(Clone, Debug)]
pub struct SourisState {
    base_location: PathBuf,
    dbs: Arc<Mutex<HashMap<String, Store>>>, //only at runtime
}

impl SourisState {
    ///returns whether the key already existed
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn new_db(&self, key: String) -> Result<bool, SourisError> {
        let mut dbs = self.dbs.lock().await;

        println!("Here");

        if dbs.contains_key(&key) {
            trace!(
                ?key,
                "Tried to add new store, found existing store with name."
            );
            return Ok(true);
        }

        dbs.insert(key.clone(), Store::default());

        let blank = Store::default().ser()?;
        let file_name = self.base_location.join(format!("{key}.sdb"));
        let mut file = File::create(&file_name).await?;
        info!(?file_name, "Writing blank SDB");

        file.write_all(&blank).await?;

        Ok(false)
    }

    ///returns whether it cleared a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn clear_db(&self, key: String) -> bool {
        let mut dbs = self.dbs.lock().await;
        if let Some(store) = dbs.get_mut(&key) {
            store.clear();
            true
        } else {
            trace!("Unable to find store.");
            false
        }
    }

    ///returns whether it removed a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn remove_db(&self, key: String) -> Result<bool, tokio::io::Error> {
        let mut dbs = self.dbs.lock().await;
        if !dbs.contains_key(&key) {
            return Ok(false);
        }

        dbs.remove(&key);

        let file_name = self.base_location.join(format!("{key}.sdb"));
        tokio::fs::remove_file(file_name).await?;

        Ok(true)
    }

    pub async fn get_db(&self, name: String) -> Option<Store> {
        let dbs = self.dbs.lock().await;
        dbs.get(&name).cloned()
    }
}

impl SourisState {
    pub async fn new() -> color_eyre::Result<Self> {
        #[tracing::instrument(level = "trace")]
        async fn get_store(location: PathBuf) -> color_eyre::Result<Store> {
            let mut file = match File::open(&location).await {
                Ok(f) => f,
                Err(e) => {
                    return if e.kind() == ErrorKind::NotFound {
                        trace!(?location, "File not found, getting empty store.");
                        return Ok(Store::default());
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

            let mut cursor = Cursor::new(&contents);
            Ok(Store::deser(&mut cursor)?)
        }

        #[tracing::instrument(level = "trace", skip(meta))]
        async fn get_internal_stores(
            meta: &Store,
            base: PathBuf,
        ) -> Option<HashMap<String, Store>> {
            let Some(Value::Array(Array(values))) =
                meta.get(&Value::String(DB_FILE_NAMES_KEY.into()))
            else {
                trace!("Unable to find existing databases - using none");
                return None;
            };

            let mut dbs = HashMap::new();

            for val in values {
                let Value::String(file_name) = val else {
                    trace!(?val, "Found non-string inside existing databases list");
                    continue;
                };

                match get_store(base.join(file_name).join(".sdb")).await {
                    Ok(s) => {
                        dbs.insert(file_name.to_owned(), s);
                    }
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
                meta.insert(
                    Value::String(DB_FILE_NAMES_KEY.into()),
                    Value::Array(Array(vec![])),
                );
                HashMap::default()
            }
        };

        let s = Self {
            base_location,
            dbs: Arc::new(Mutex::new(dbs)),
        };

        s.save().await?;

        Ok(s)
    }

    pub async fn save(&self) -> color_eyre::Result<()> {
        let mut names = vec![];
        for (name, db) in self.dbs.lock().await.iter() {
            let file_name = self.base_location.join(format!("{name}.sdb"));
            let bytes = db.ser()?;
            trace!(?file_name, "Writing out DB");
            if let Err(e) = write_to_file(&bytes, file_name, &self.base_location).await {
                error!(?e, "Error writing out database");
            } else {
                names.push(Value::String(name.to_string()));
            }
        }

        let mut meta = Store::default();
        meta.insert(
            Value::String(DB_FILE_NAMES_KEY.to_string()),
            Value::Array(Array(names)),
        );

        let location = self.base_location.join(META_DB_FILE_NAME);
        let meta = meta.ser()?;
        debug!(bytes=?meta.len(), "Writing metadata to file");
        write_to_file(&meta, location, &self.base_location).await
    }
}

async fn write_to_file(
    bytes: &[u8],
    path: impl AsRef<Path> + Debug,
    base_location: impl AsRef<Path> + Debug,
) -> color_eyre::Result<()> {
    let mut metadata_file = match File::create(&path).await {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                //folder must not exist
                trace!(?path, ?base_location, "Unable to find folder, creating");

                create_dir_all(&base_location).await?;
                File::create(path).await?
            } else {
                return Err(e.into());
            }
        }
    };

    metadata_file.write_all(bytes).await?;

    Ok(())
}
