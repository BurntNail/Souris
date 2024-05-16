pub const ACTION_KEY: &str = "action";

pub const DB_NAME_KEY: &str = "name";
pub const OVERWRITE_EXISTING_DB_KEY: &str = "overwrite_existing";
pub const KEY_KEY: &str = "key";
pub const VALUE_KEY: &str = "value";

pub const GET_ALL_DB_NAMES_KEY: &str = "db_names";
pub const ADD_DB_CONTENT_CONTENT_KEY: &str = "content";

pub const RESPONSE_STATUS_CODE_KEY: &str = "status_code";
pub const RESPONSE_BODY_KEY: &str = "body";

#[derive(Debug)]
pub enum Action {
    GetDb,
    GetAllDbNames,
    AddDatabase,
    AddDatabaseWithContent,
    RemoveDatabase,
    ClearDatabase,
    AddKeyValue,
    RemoveKeyValue,
    GetValue,
}

impl From<Action> for u8 {
    fn from(value: Action) -> Self {
        match value {
            Action::GetDb => 0,
            Action::GetAllDbNames => 1,
            Action::AddDatabase => 2,
            Action::AddDatabaseWithContent => 3,
            Action::RemoveDatabase => 4,
            Action::ClearDatabase => 5,
            Action::AddKeyValue => 6,
            Action::RemoveKeyValue => 7,
            Action::GetValue => 8,
        }
    }
}
impl TryFrom<u8> for Action {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Action::GetDb),
            1 => Ok(Action::GetAllDbNames),
            2 => Ok(Action::AddDatabase),
            3 => Ok(Action::AddDatabaseWithContent),
            4 => Ok(Action::RemoveDatabase),
            5 => Ok(Action::ClearDatabase),
            6 => Ok(Action::AddKeyValue),
            7 => Ok(Action::RemoveKeyValue),
            8 => Ok(Action::GetValue),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum ResponseRequired {
    SendMore,
}

impl From<ResponseRequired> for u8 {
    fn from(value: ResponseRequired) -> Self {
        match value {
            ResponseRequired::SendMore => 0,
        }
    }
}
impl TryFrom<u8> for ResponseRequired {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ResponseRequired::SendMore),
            _ => Err(()),
        }
    }
}

//I could do something fancy - like the first two bits signify the CRUD, the third bit a database or a value etc
//but I don't need to, so i won't ;)
