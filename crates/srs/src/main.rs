#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::{Parser, Subcommand};
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Confirm, Error as DError, FuzzySelect, Input,
};
use sourisdb::{
    store::{Store, StoreSerError},
    values::{ValueSerError},
    utilities::value_utils::get_value_from_stdin,
};
use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{Error as IOError, ErrorKind, Read, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(version, author)]
struct Args {
    path: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    CreateNew,
    AddEntry,
    ViewAll,
    #[cfg(debug_assertions)]
    DebugViewAll,
    RemoveEntry,
    UpdateEntry,
    ExportToJSON {
        json_location: PathBuf,
    },
    CreateNewFromJSON {
        json_location: PathBuf,
    },
}

fn main() {
    if let Err(e) = fun_main(Args::parse()) {
        eprintln!("Error running program: {e}");
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
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "Error handling IO: {e}"),
            Error::Dialoguer(e) => write!(f, "Error with dialoguer: {e}"),
            Error::SerdeJson(e) => write!(f, "Error with JSON: {e}"),
            Error::Value(e) => write!(f, "Error with values: {e}"),
            Error::Store(e) => write!(f, "Error with store: {e}"),
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

#[allow(clippy::too_many_lines)]
fn fun_main(Args { path, command }: Args) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    match command {
        Commands::CreateNew => {
            new_store_in_file(path, &theme)?;
        }
        Commands::ViewAll => {
            let store = view_all(path, &theme)?;

            println!("{store}");
        }
        #[cfg(debug_assertions)]
        Commands::DebugViewAll => {
            let store = view_all(path, &theme)?;
            println!("{store:#?}");
        }
        Commands::CreateNewFromJSON { json_location } => {
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
            println!("Successfully parsed JSON");

            let mut output = File::create(path)?;
            output.write_all(&store_bytes)?;
        }
        Commands::AddEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            let key = Input::with_theme(&theme).with_prompt("Key: ").interact()?;
            let value = get_value_from_stdin("Value: ", &theme)?;

            println!();

            println!("Received Key: {key}");
            println!("Received Value: {value}");

            println!();

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Addition?")
                .interact()?
            {
                store.insert(key, value);
                let mut file = File::create(path)?;
                file.write_all(&store.ser()?)?;

                println!("Successfully added to store.");
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }
        }
        Commands::RemoveEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to be removed:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key).clone(); //idc if it gets swapped as we drop it next
            drop(keys);

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Removal?")
                .interact()?
            {
                match store.remove(&key) {
                    Some(value) => {
                        let mut file = File::create(path)?;
                        file.write_all(&store.ser()?)?;

                        println!("Successfully removed {value:?} from store.");
                    }
                    None => {
                        println!("Key not found. Nothing removed.");
                    }
                }
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }
        }
        Commands::UpdateEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            println!();

            let mut keys = store.keys().collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to update value of:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key).clone(); //idc if it gets swapped as we drop keys next and swapping is faster
            drop(keys);

            if !Confirm::with_theme(&theme)
                .with_prompt(format!("Confirm edit key {key}?"))
                .interact()?
            {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }

            let existing = &store[&key];
            let new = get_value_from_stdin("Enter the new value: ", &theme)?;
            if !Confirm::with_theme(&theme)
                .with_prompt(format!("Confirm replace value {existing} with {new}?"))
                .interact()?
            {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }

            store.insert(key, new);

            let mut file = File::create(path)?;
            file.write_all(&store.ser()?)?;

            println!("Successfully updated");
        }
        Commands::ExportToJSON { json_location } => {
            let store = view_all(path, &theme)?;
            let json = serde_json::to_string(&store)?;

            let mut file = File::create(json_location)?;
            file.write_all(json.as_bytes())?;
        }
    }

    Ok(())
}

fn view_all(path: PathBuf, theme: &dyn Theme) -> Result<Store, Error> {
    match File::open(path.clone()) {
        Err(e) if e.kind() == ErrorKind::NotFound => {
            if Confirm::with_theme(theme)
                .with_prompt("No file found. Create new?")
                .interact()?
            {
                new_store_in_file(path, theme)
            } else {
                println!("File not created. Exiting...");
                std::process::exit(0);
            }
        }
        Err(e) => Err(e.into()),
        Ok(mut file) => {
            let mut contents: Vec<u8> = vec![];
            {
                let mut tmp = [0_u8; 128];
                loop {
                    match file.read(&mut tmp)? {
                        0 => break,
                        n => contents.extend(&tmp[0..n]),
                    }
                }
            }

            let store = Store::deser(&contents)?;
            Ok(store)
        }
    }
}

fn new_store_in_file(path: PathBuf, theme: &dyn Theme) -> Result<Store, Error> {
    let mut file = match File::create_new(path.clone()) {
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            if Confirm::with_theme(theme)
                .with_prompt("File already exists. Continue & Overwrite?")
                .interact()?
            {
                File::create(path)?
            } else {
                println!("File not overwritten. Exiting...");
                std::process::exit(0);
            }
        }
        Err(e) => return Err(e.into()),
        Ok(f) => f,
    };

    let store = Store::new();
    file.write_all(&store.ser()?)?;
    println!("Successfully created new SourisDB.");

    Ok(store)
}
