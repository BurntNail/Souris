use axum::http::StatusCode;
use color_eyre::eyre::bail;
use dirs::data_dir;
use sourisdb::{store::Store, values::Value};
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
use meta::{DB_FILE_NAMES_KEY, META_DB_FILE_NAME};

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SourisState {
    base_location: PathBuf,
    dbs: Arc<Mutex<HashMap<String, Store>>>, //only at runtime
}

impl SourisState {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn new_db(
        &self,
        name: String,
        overwrite_existing: bool,
    ) -> Result<StatusCode, SourisError> {
        if &name == "meta" || !name.is_ascii() {
            return Err(SourisError::InvalidDatabaseName);
        }

        let mut dbs = self.dbs.lock().await;

        if dbs.contains_key(&name) {
            trace!(
                ?name,
                "Tried to add new store, found existing store with name."
            );

            if overwrite_existing {
                let Some(db) = dbs.get_mut(&name) else {
                    unreachable!("just checked that the key exists")
                };
                db.clear();
            }
            return Ok(StatusCode::OK);
        }

        dbs.insert(name.clone(), Store::new());

        Ok(StatusCode::CREATED)
    }

    #[tracing::instrument(level = "trace", skip(self, contents))]
    pub async fn new_db_with_contents(
        &self,
        name: String,
        overwrite_existing: bool,
        contents: Store,
    ) -> StatusCode {
        let mut stores = self.dbs.lock().await;

        let mut contained = false;
        if stores.contains_key(&name) {
            contained = true;
            if !overwrite_existing {
                return StatusCode::OK;
            }
        }

        stores.insert(name, contents);

        if contained {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        }
    }

    ///returns whether it cleared a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn clear_db(&self, name: String) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;
        if let Some(store) = dbs.get_mut(&name) {
            store.clear();
            Ok(())
        } else {
            trace!("Unable to find store.");
            Err(SourisError::DatabaseNotFound)
        }
    }

    ///returns whether it removed a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn remove_db(&self, name: String) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;
        if !dbs.contains_key(&name) {
            return Err(SourisError::DatabaseNotFound);
        }

        dbs.remove(&name);

        let file_name = self.base_location.join(format!("{name}.sdb"));

        if let Err(e) = tokio::fs::remove_file(file_name).await {
            if e.kind() != ErrorKind::NotFound {
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn get_db(&self, name: String) -> Result<Store, SourisError> {
        let dbs = self.dbs.lock().await;
        dbs.get(&name).cloned().ok_or(SourisError::DatabaseNotFound)
    }

    pub async fn add_key_value_pair(&self, db: String, k: String, v: Value) {
        let mut dbs = self.dbs.lock().await;

        let db = if let Some(d) = dbs.get_mut(&db) {
            d
        } else {
            dbs.insert(db.clone(), Store::default());
            dbs.get_mut(&db).expect("just added this key")
        };

        db.insert(k, v);
    }

    pub async fn get_value(&self, db: String, k: &String) -> Result<Value, SourisError> {
        let dbs = self.dbs.lock().await;

        let Some(db) = dbs.get(&db) else {
            return Err(SourisError::DatabaseNotFound);
        };
        let Some(key) = db.get(k).cloned() else {
            return Err(SourisError::KeyNotFound);
        };

        Ok(key)
    }

    pub async fn rm_key(&self, db: String, key: String) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;

        let Some(db) = dbs.get_mut(&db) else {
            return Err(SourisError::DatabaseNotFound);
        };

        match db.remove(&key) {
            Some(_) => Ok(()),
            None => Err(SourisError::DatabaseNotFound),
        }
    }

    pub async fn get_all_db_names(&self) -> Vec<String> {
        self.dbs.lock().await.keys().cloned().collect()
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
                        return Ok(Store::new());
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

        #[tracing::instrument(level = "trace", skip(meta))]
        async fn get_internal_stores(
            meta: &Store,
            base: PathBuf,
        ) -> Option<HashMap<String, Store>> {
            let Some(Value::Array(values)) = meta.get(DB_FILE_NAMES_KEY) else {
                trace!("Unable to find existing databases.");
                return None;
            };

            let mut dbs = HashMap::new();

            for val in values {
                let Some(file_name) = val.as_str() else {
                    trace!(?val, "Found non-string inside existing databases list");
                    continue;
                };

                match get_store(base.join(format!("{file_name}.sdb"))).await {
                    Ok(s) => {
                        dbs.insert(file_name.to_string(), s);
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

        let dbs = if let Some(dbs) = get_internal_stores(&meta, base_location.clone()).await {
            dbs
        } else {
            meta.insert(DB_FILE_NAMES_KEY.into(), Value::Array(vec![]));
            HashMap::default()
        };

        let s = Self {
            base_location,
            dbs: Arc::new(Mutex::new(dbs)),
        };

        Ok(s)
    }

    pub async fn save(&self) -> color_eyre::Result<()> {
        let mut names = vec![];

        for (name, db) in self.dbs.lock().await.iter() {
            let file_name = self.base_location.join(format!("{name}.sdb"));
            let bytes = db.ser()?;

            if let Err(e) = write_to_file(&bytes, file_name, &self.base_location).await {
                error!(?e, "Error writing out database");
            } else {
                names.push(Value::String(name.to_string()));
            }
        }

        let mut meta = Store::new();
        meta.insert(DB_FILE_NAMES_KEY.into(), Value::Array(names));

        let location = self.base_location.join(META_DB_FILE_NAME);
        let meta = meta.ser()?;
        write_to_file(&meta, location, &self.base_location).await
    }
}

async fn write_to_file(
    bytes: &[u8],
    path: impl AsRef<Path> + Debug,
    base_location: impl AsRef<Path> + Debug,
) -> color_eyre::Result<()> {
    let mut file = match File::create(&path).await {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                //folder must not exist, hopefully?
                trace!(?path, ?base_location, "Unable to find folder, creating");

                create_dir_all(&base_location).await?;
                File::create(path).await?
            } else {
                return Err(e.into());
            }
        }
    };

    file.write_all(bytes).await?;

    Ok(())
}
