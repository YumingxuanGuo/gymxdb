#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod expression;
pub use expression::Expression;

use serde_derive::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A datatype
#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    Integer,
    Float,
    String,
}

/// A specific value of a data type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

/// A row of values
pub type Row = Vec<Value>;

/// A row iterator
pub type Rows = Box<dyn Iterator<Item = Result<Row>> + Send>;