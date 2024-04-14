use std::fmt::{Display, Formatter};
use std::fs::{File};
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use std::io::{Error as IOError, ErrorKind, Read, Write};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};
use dialoguer::{Confirm, Error as DError, FuzzySelect, Input};
use dialoguer::theme::{ColorfulTheme, Theme};
use daddy::niches::integer::Integer;
use daddy::store::{Store, StoreError};
use daddy::values::{Value, ValueTy};

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
    RemoveEntry,
}

fn main () {
    if let Err(e) = fun_main(Args::parse()) {
        eprintln!("Error running program: {e:?}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum Error {
    IO(IOError),
    Store(StoreError),
    Dialoguer(DError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "Error handling IO: {e:?}"),
            Error::Store(e) => write!(f, "Error in store: {e:?}"),
            Error::Dialoguer(e) => write!(f, "Error with dialoguer: {e:?}"),
        }
    }
}

impl From<IOError> for Error {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<StoreError> for Error {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}
impl From<DError> for Error {
    fn from(value: DError) -> Self {
        Self::Dialoguer(value) //yes, i'm aware that this is a wrapper over IOError, but just in case :)
    }
}

fn fun_main (Args { path, command }: Args) -> Result<(), Error>{
    let theme = ColorfulTheme::default();

    match command {
        Commands::CreateNew => {
            new_store_in_file(path, &theme)?;
        }
        Commands::ViewAll => {
            let store = view_all(path, &theme)?;
            
            println!("Version: {:?}", store.version());

            let mut table = Table::new();
            table.set_header(vec!["Key", "Value"])
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::Dynamic);

            for (k, v) in store.into_iter() {
                table.add_row(vec![format!("{k}"), format!("{v}")]);
            }
            println!("{table}");
        },
        Commands::AddEntry => {
            let mut store = view_all(path.clone(), &theme)?;
            
            let key = get_value_from_stdin("Please enter the key:", &theme)?;
            let value = get_value_from_stdin("Please enter the value:", &theme)?;
            
            println!();

            println!("Received Key: {key}");
            println!("Received Value: {value}");
            
            println!();

            if Confirm::with_theme(&theme).with_prompt("Confirm Addition?").interact()? {
                store.insert(key, value);
                let mut file = File::create(path)?;
                file.write_all(&store.ser()?)?;

                println!("Successfully added to store.");
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }
        },
        Commands::RemoveEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            println!();

            let key = get_value_from_stdin("Please enter the key to be removed:", &theme)?;

            println!("Received Key: {key}");

            if Confirm::with_theme(&theme).with_prompt("Confirm Removal?").interact()? {
                match store.remove(&key) {
                    Some(value) => {
                        let mut file = File::create(path)?;
                        file.write_all(&store.ser()?)?;

                        println!("Successfully removed {value:?} from store.");
                    },
                    None => {
                        println!("Key not found. Nothing removed.");
                    }
                }
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }

        }
    }

    Ok(())
}

fn get_value_from_stdin (prompt: impl Display, theme: &dyn Theme) -> Result<Value, Error> {
    println!("{prompt}");

    let tys = [ValueTy::Bool, ValueTy::Int, ValueTy::Ch, ValueTy::String, ValueTy::Binary];
    let selection = FuzzySelect::with_theme(theme)
        .with_prompt("Which type?")
        .items(tys.into_iter().map(|x| format!("{x:?}")).collect::<Vec<_>>().as_slice())
        .interact()?;
    match tys[selection] {
        ValueTy::Ch => {
            let ch: char = Input::with_theme(theme).with_prompt("Which character?").interact()?;
            Ok(Value::Ch(ch))
        },
        ValueTy::String => {
            let st: String = Input::with_theme(theme).with_prompt("What text?").interact()?;
            Ok(Value::String(st))
        },
        ValueTy::Binary => {
            let st: String = Input::with_theme(theme).with_prompt("What text to be interpreted as UTF-8 bytes?").interact()?;
            Ok(Value::Binary(st.as_bytes().to_vec()))
        },
        ValueTy::Bool => {
            let b = FuzzySelect::with_theme(theme).items(&["False", "True"]).interact()?;
            Ok(Value::Bool(b != 0))
        },
        ValueTy::Int => {
            let i: i64 = Input::with_theme(theme).with_prompt("Which number?").interact()?;
            Ok(Value::Int(Integer::from(i)))
        },
    }
}

fn view_all(path: PathBuf, theme: &dyn Theme) -> Result<Store, Error> {
    match File::open(path.clone()) {
        Err(e) if e.kind() == ErrorKind::NotFound => {
            if Confirm::with_theme(theme)
                .with_prompt("No file found. Create new?")
                .interact()? {
                new_store_in_file(path, theme)
            } else {
                println!("File not created. Exiting...");
                std::process::exit(0);
            }
        },
        Err(e) => Err(e.into()),
        Ok(mut file) => {
            let mut contents: Vec<u8> = vec![];
            {
                let mut tmp = [0_u8; 128];
                loop {
                    match file.read(&mut tmp)? {
                        0 => break,
                        n => contents.extend(&tmp[0..n])
                    }
                }
            }
            
            println!("Read {} bytes.", contents.len()); //grammar: always != 1
            
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
                .interact()? {
                File::create(path)?
            } else {
                println!("File not overwritten. Exiting...");
                std::process::exit(0);
            }
        },
        Err(e) => return Err(e.into()),
        Ok(f) => f,
    };
    
    let store = Store::new();
    file.write_all(&store.ser()?)?;
    println!("Successfully created new DDB.");

    Ok(store)
}