pub enum Value {
    Ch(char),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Int(i64)
}

pub enum ValueFailure {

}

impl Value {
    pub fn serialise (self, lookup_index: usize) -> Vec<u8> {
        let mut res = vec![];


        let ty: u8 = match &self {
            Self::Ch(_) =>     0b0000,
            Self::String(_) => 0b0001,
            Self::Binary(_) => 0b0010,
            Self::Bool(_)   => 0b0011,
            Self::Int(_)    => 0b0100,
        };

        todo!()
    }
}

pub enum Key {
    String(String)
}