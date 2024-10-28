#![warn(clippy::all, clippy::pedantic)]

use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{Error as IOError, Read, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Confirm, Error as DError, FuzzySelect, Input,
};

use crate::value_utils::get_value_from_stdin;
use sourisdb::{
    client::{ClientError, SyncClient},
    store::{Store, StoreSerError},
    values::ValueSerError,
};

mod value_utils;

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
        #[arg(short, long)]
        add_souris_types: bool,
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
    NoDatabasesFound,
    Client(Box<ClientError>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "Error handling IO: {e}"),
            Error::Dialoguer(e) => write!(f, "Error with dialoguer: {e}"),
            Error::SerdeJson(e) => write!(f, "Error with JSON: {e}"),
            Error::Value(e) => write!(f, "Error with values: {e}"),
            Error::Store(e) => write!(f, "Error with store: {e}"),
            Error::NoDatabasesFound => write!(f, "No databases found"),
            Error::Client(e) => write!(f, "Error with souris client: {e}"),
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
impl From<ClientError> for Error {
    fn from(value: ClientError) -> Self {
        Self::Client(Box::new(value))
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::Dialoguer(e) => Some(e),
            Error::SerdeJson(e) => Some(e),
            Error::Value(e) => Some(e),
            Error::Store(e) => Some(e),
            Error::Client(e) => Some(e),
            Error::NoDatabasesFound => None,
        }
    }
}

#[allow(clippy::collapsible_if, clippy::too_many_lines)]
fn fun_main(Arguments { path, command }: Arguments) -> Result<(), Error> {
    let theme = ColorfulTheme::default();
    let client = SyncClient::new(path.clone(), 7687)?;

    match command {
        Commands::CreateNew { db_name } => {
            let all = client.get_all_dbs()?;

            if all.contains(&db_name) {
                if !Confirm::with_theme(&theme)
                    .with_prompt("A database with that name already exists. Clear that database?")
                    .interact()?
                {
                    println!("Cancelled clearing database");
                    return Ok(());
                }
            }

            if client.create_new_db(true, &db_name)? {
                println!("Database successfully created");
            } else {
                println!("Successfully cleared database");
            }
        }
        Commands::ViewAll => {
            let (_, store) = pick_db(&client, &theme)?;
            println!("{store}");
        }
        #[cfg(debug_assertions)]
        Commands::DebugViewAll => {
            let (_, store) = pick_db(&client, &theme)?;
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

            let store = Store::from_json_bytes(&bytes)?;
            let db_name = pick_db_name(true, &client, &theme)?;

            if client.add_db_with_contents(true, &db_name, &store)? {
                println!("Created new database with JSON.");
            } else {
                println!("Overwrote existing database with JSON.");
            }
        }
        Commands::AddEntry => {
            let db_name = pick_db_name(true, &client, &theme)?;

            let key: String = Input::with_theme(&theme).with_prompt("Key: ").interact()?;
            let value = get_value_from_stdin("Value: ", &theme)?;

            println!();

            println!("Received Key: {key}");
            println!("Received Value: {value}");

            println!();

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Addition?")
                .interact()?
            {
                if client.add_entry_to_db(&db_name, &key, &value)? {
                    println!("Successfully created new key-value pair.");
                } else {
                    println!("Successfully overwrote existing key-value pair.");
                }
            } else {
                println!("Cancelled addition");
            }
        }
        Commands::RemoveEntry => {
            let (db_name, store) = pick_db(&client, &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();

            if keys.is_empty() {
                println!("Database already empty.");
            } else {
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
                    client.remove_entry_from_db(&db_name, &key)?;
                    println!("Successfully removed Key");
                } else {
                    println!("Cancelled removing database.");
                }
            }
        }
        Commands::UpdateEntry => {
            let (db_name, store) = pick_db(&client, &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();

            if keys.is_empty() {
                println!("Database is empty.");
            } else {
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
                    if !client.add_entry_to_db(&db_name, &key, &new_val)? {
                        println!("Successfully overwrote existing key-value pair.");
                    }
                } else {
                    println!("Cancelled updating key-value pair.");
                }
            }
        }
        Commands::ExportToJSON {
            json_location,
            add_souris_types,
        } => {
            let (name, store) = pick_db(&client, &theme)?;
            println!("Received Database {name:?}, converting to JSON");

            match store.to_json(add_souris_types) {
                Some(json) => {
                    let json = serde_json::to_string_pretty(&json)?;

                    println!("Converted to JSON, writing to {json_location:?}");

                    let mut file = File::create(json_location)?;
                    file.write_all(json.as_bytes())?;
                }
                None => {
                    eprintln!("Unable to convert to JSON - ensure there are no NaN/infinite floats or integers which cannot fit into the range from i64::MIN to u64::MAX");
                }
            }
        }
        Commands::RemoveDatabase => {
            let db_name = pick_db_name(false, &client, &theme)?;
            client.remove_db(&db_name)?;
            println!("Successfully removed database");
        }
    }

    Ok(())
}

fn pick_db(client: &SyncClient, theme: &dyn Theme) -> Result<(String, Store), Error> {
    let chosen_db_name = pick_db_name(false, client, theme)?;
    let chosen_store = client.get_store(&chosen_db_name)?;
    Ok((chosen_db_name, chosen_store))
}

#[allow(clippy::collapsible_if)]
fn pick_db_name(
    can_pick_new: bool,
    client: &SyncClient,
    theme: &dyn Theme,
) -> Result<String, Error> {
    let mut names = client.get_all_dbs()?;

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
    if names.len() == 1 {
        let Some(first) = names.pop() else {
            unreachable!()
        };
        return Ok(first);
    }

    let chosen_index = FuzzySelect::with_theme(theme)
        .items(&names)
        .with_prompt("Which database?")
        .interact()?;
    Ok(names.swap_remove(chosen_index))
}
