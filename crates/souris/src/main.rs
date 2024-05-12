use clap::{Parser, Subcommand};
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Error as DError, FuzzySelect,
};
use sourisdb::{
    hashbrown::HashMap,
    serde_json::Value as SJValue,
    store::{Store, StoreSerError},
    values::{ValueSerError},
};
use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{Error as IOError, Read},
    path::PathBuf,
};
use std::net::Ipv4Addr;
use reqwest::blocking::Client;

#[derive(Parser, Debug)]
#[command(version, author)]
struct Arguments {
    path: String,
    db_name: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
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
    ImportFromJSON {
        json_location: PathBuf,
    },
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

fn fun_main(Arguments { path, db_name, command }: Arguments) -> Result<(), Error> {
    let theme = ColorfulTheme::default();
    let client = Client::new();

    match command {
        Commands::CreateNew => {
            let new_store = new_store(path, client, db_name, &theme)?;
            println!("Created new store: {new_store}");
        }
        Commands::ViewAll => {
            let store = get_all(path, client, db_name)?;
            println!("{store}");
        }
        #[cfg(debug_assertions)]
        Commands::DebugViewAll => {
            let store = get_all(path, client, db_name)?;
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

            let overwrite_existing = FuzzySelect::with_theme(&theme).items(&["Overwrite", "Ignore"]).with_prompt("Existing Database: ").interact()? == 0;

            let mut args = HashMap::new();
            args.insert("overwrite_existing", SJValue::Bool(overwrite_existing));
            args.insert("name", SJValue::String(db_name.clone()));

            let rsp = client.put(format!("http://{path}:2256/v1/add_db_with_content")).query(&args).body(store_bytes).send()?.error_for_status()?;

            println!("Got contents: {rsp:?}");
        }
        Commands::AddEntry => {
            /*let mut store = view_all(store_location.clone(), &theme)?;

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
                let mut file = File::create(store_location)?;
                file.write_all(&store.ser()?)?;

                println!("Successfully added to store.");
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }*/
            todo!()
        }
        Commands::RemoveEntry => {
            /*let mut store = view_all(store_location.clone(), &theme)?;

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
                        let mut file = File::create(store_location)?;
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
            }*/
            todo!()
        }
        Commands::UpdateEntry => {
            /*let mut store = view_all(store_location.clone(), &theme)?;

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

            let mut file = File::create(store_location)?;
            file.write_all(&store.ser()?)?;

            println!("Successfully updated");*/
            todo!()
        }
        Commands::ExportToJSON { json_location } => {
            /*let store = view_all(store_location, &theme)?;
            let json = serde_json::to_string(&store)?;

            let mut file = File::create(json_location)?;
            file.write_all(json.as_bytes())?;*/
            todo!()
        }
    }

    Ok(())
}

fn get_all(path: String, client: Client, db_name: String) -> Result<Store, Error> {
    let mut args = HashMap::new();
    args.insert("name", SJValue::String(db_name));

    let rsp = client.get(format!("http://{path}:2256/v1/get_db")).query(&args).send()?.error_for_status()?.bytes()?;
    let store = Store::deser(rsp.as_ref())?;

    Ok(store)
}

fn new_store(path: String, client: Client, db_name: String, theme: &dyn Theme) -> Result<Store, Error> {
    let overwrite_existing = FuzzySelect::with_theme(theme).items(&["Overwrite", "Ignore"]).with_prompt("Existing Database: ").interact()? == 0;

    let mut args = HashMap::new();
    args.insert("overwrite_existing", SJValue::Bool(overwrite_existing));
    args.insert("name", SJValue::String(db_name.clone()));

    let rsp = client.post(format!("http://{path}:2256/v1/add_db")).query(&args).send()?.error_for_status()?;

    println!("Got response: {rsp:?}");

    get_all(path, client, db_name)
}
