#![warn(clippy::all, clippy::pedantic)]

use clap::{Parser, Subcommand};
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Confirm, Error as DError, FuzzySelect, Input,
};
use reqwest::blocking::Client;
use sourisdb::{
    hashbrown::HashMap,
    serde_json::Value as SJValue,
    store::{Store, StoreSerError},
    utilities::value_utils::get_value_from_stdin,
    values::ValueSerError,
};
use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{Error as IOError, Read, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(version, author)]
struct Arguments {
    path: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    CreateNew {
        db_name: String,
    },
    AddEntry,
    ViewAll,
    #[cfg(debug_assertions)]
    DebugViewAll,
    RemoveEntry,
    UpdateEntry,
    ExportToJSON {
        json_location: PathBuf,
    },
    ImportFromJSON {
        json_location: PathBuf,
    },
    RemoveDatabase,
}

fn main() {
    if let Err(e) = fun_main(Arguments::parse()) {
        eprintln!("Error running program: {e:?}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum Error {
    IO(IOError),
    Dialoguer(DError),
    SerdeJson(serde_json::Error),
    Value(ValueSerError),
    Store(StoreSerError),
    Reqwest(reqwest::Error),
    NoDatabasesFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "Error handling IO: {e}"),
            Error::Dialoguer(e) => write!(f, "Error with dialoguer: {e}"),
            Error::SerdeJson(e) => write!(f, "Error with JSON: {e}"),
            Error::Value(e) => write!(f, "Error with values: {e}"),
            Error::Store(e) => write!(f, "Error with store: {e}"),
            Error::Reqwest(e) => write!(f, "Error with reqwest: {e}"),
            Error::NoDatabasesFound => write!(f, "No databases found"),
        }
    }
}

impl From<IOError> for Error {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<DError> for Error {
    fn from(value: DError) -> Self {
        Self::Dialoguer(value) //yes, i'm aware that this is a wrapper over IOError, but just in case :)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<ValueSerError> for Error {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}
impl From<StoreSerError> for Error {
    fn from(value: StoreSerError) -> Self {
        Self::Store(value)
    }
}
impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

#[allow(clippy::collapsible_if, clippy::too_many_lines)]
fn fun_main(Arguments { path, command }: Arguments) -> Result<(), Error> {
    let theme = ColorfulTheme::default();
    let client = Client::new();

    match command {
        Commands::CreateNew { db_name } => {
            let all = get_all_dbs(&path, &client)?;

            if all.contains(&db_name) {
                if !Confirm::with_theme(&theme)
                    .with_prompt("A database with that name already exists. Overwrite?")
                    .interact()?
                {
                    return Ok(());
                }
            }

            let mut args = HashMap::new();
            args.insert("overwrite_existing", SJValue::Bool(true));
            args.insert("name", SJValue::String(db_name.clone()));

            let rsp = client
                .post(format!("http://{path}:2256/v1/add_db"))
                .query(&args)
                .send()?
                .error_for_status()?;

            println!("Got response: {rsp:?}");
        }
        Commands::ViewAll => {
            let (_, store) = pick_db(&path, &client, &theme)?;
            println!("{store}");
        }
        #[cfg(debug_assertions)]
        Commands::DebugViewAll => {
            let (_, store) = pick_db(&path, &client, &theme)?;
            println!("{store:#?}");
        }
        Commands::ImportFromJSON { json_location } => {
            let mut file = File::open(json_location)?;
            let mut bytes = vec![];
            let mut tmp = [0_u8; 128];
            loop {
                match file.read(&mut tmp)? {
                    0 => break,
                    n => {
                        bytes.extend(&tmp[0..n]);
                    }
                }
            }

            let store = Store::from_json(&bytes)?;
            let store_bytes = store.ser()?;

            let db_name = pick_db_name(true, &path, &client, &theme)?;

            let mut args = HashMap::new();
            args.insert("overwrite_existing", SJValue::Bool(true));
            args.insert("name", SJValue::String(db_name.clone()));

            let rsp = client
                .put(format!("http://{path}:2256/v1/add_db_with_content"))
                .query(&args)
                .body(store_bytes)
                .send()?
                .error_for_status()?;

            println!("Got contents: {rsp:?}");
        }
        Commands::AddEntry => {
            let key = Input::with_theme(&theme).with_prompt("Key: ").interact()?;
            let value = get_value_from_stdin("Value: ", &theme)?;

            println!();

            println!("Received Key: {key}");
            println!("Received Value: {value}");

            println!();

            let db_name = pick_db_name(true, &path, &client, &theme)?;

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Addition?")
                .interact()?
            {
                let mut args = HashMap::new();
                args.insert("db", SJValue::String(db_name));
                args.insert("key", SJValue::String(key));

                let bytes = value.ser()?;

                let rsp = client
                    .put(format!("http://{path}:2256/v1/add_kv"))
                    .query(&args)
                    .body(bytes)
                    .send()?
                    .error_for_status()?;
                println!("Got response: {rsp:?}");
            }
        }
        Commands::RemoveEntry => {
            let (db_name, store) = pick_db(&path, &client, &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to be removed:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key).clone(); //idc if it gets swapped as we drop it next

            drop(keys);
            drop(store);

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Removal?")
                .interact()?
            {
                let mut args = HashMap::new();
                args.insert("db", SJValue::String(db_name));
                args.insert("key", SJValue::String(key));

                let rsp = client
                    .post(format!("http://{path}:2256/v1/rm_key"))
                    .query(&args)
                    .send()?
                    .error_for_status()?;
                println!("Got response: {rsp:?}");
            }
        }
        Commands::UpdateEntry => {
            let (db_name, store) = pick_db(&path, &client, &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to be updated:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key).clone(); //idc if it gets swapped as we drop it next

            drop(keys);
            drop(store);

            let new_val = get_value_from_stdin("New Value: ", &theme)?;

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Update?")
                .interact()?
            {
                let mut args = HashMap::new();
                args.insert("db", SJValue::String(db_name));
                args.insert("key", SJValue::String(key));

                let bytes = new_val.ser()?;

                let rsp = client
                    .put(format!("http://{path}:2256/v1/add_kv"))
                    .query(&args)
                    .body(bytes)
                    .send()?
                    .error_for_status()?;
                println!("Got response: {rsp:?}");
            }
        }
        Commands::ExportToJSON { json_location } => {
            let (name, store) = pick_db(&path, &client, &theme)?;
            println!("Received Database {name:?}, converting to JSON");

            let json = serde_json::to_string_pretty(&store)?;
            println!("Converted to JSON, writing to {json_location:?}");

            let mut file = File::create(json_location)?;
            file.write_all(json.as_bytes())?;
        }
        Commands::RemoveDatabase => {
            let db_name = pick_db_name(false, &path, &client, &theme)?;
            let mut args = HashMap::new();
            args.insert("name", SJValue::String(db_name));

            let rsp = client
                .post(format!("http://{path}:2256/v1/rm_db"))
                .query(&args)
                .send()?
                .error_for_status()?;
            println!("Got response: {rsp:?}");
        }
    }

    Ok(())
}

fn get_store(path: &str, client: &Client, db_name: String) -> Result<Store, Error> {
    let mut args = HashMap::new();
    args.insert("name", SJValue::String(db_name));

    let rsp = client
        .get(format!("http://{path}:2256/v1/get_db"))
        .query(&args)
        .send()?
        .error_for_status()?
        .bytes()?;
    let store = Store::deser(rsp.as_ref())?;

    Ok(store)
}

fn get_all_dbs(path: &str, client: &Client) -> Result<Vec<String>, reqwest::Error> {
    client
        .get(format!("http://{path}:2256/v1/get_all_dbs"))
        .send()?
        .error_for_status()?
        .json()
}

fn pick_db(path: &str, client: &Client, theme: &dyn Theme) -> Result<(String, Store), Error> {
    let chosen_db_name = pick_db_name(false, path, client, theme)?;
    let chosen_store = get_store(path, client, chosen_db_name.clone())?;
    Ok((chosen_db_name, chosen_store))
}

#[allow(clippy::collapsible_if)]
fn pick_db_name(
    can_pick_new: bool,
    path: &str,
    client: &Client,
    theme: &dyn Theme,
) -> Result<String, Error> {
    let mut names = get_all_dbs(path, client)?;

    if can_pick_new {
        if Confirm::with_theme(theme)
            .with_prompt("New Database?")
            .interact()?
        {
            return Ok(loop {
                let trial = Input::with_theme(theme)
                    .with_prompt("Database Name: ")
                    .interact()?;
                if &trial == "meta" {
                    println!("That name is reserved.");
                    continue;
                }

                if !names.contains(&trial) {
                    break trial;
                }

                if Confirm::with_theme(theme)
                    .with_prompt("This database already exists. Overwrite?")
                    .interact()?
                {
                    break trial;
                }
            });
        }
    }

    if names.is_empty() {
        return Err(Error::NoDatabasesFound);
    }

    let chosen_index = FuzzySelect::with_theme(theme)
        .items(&names)
        .with_prompt("Which database?")
        .interact()?;
    Ok(names.swap_remove(chosen_index))
}
