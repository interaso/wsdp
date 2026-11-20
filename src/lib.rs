#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod client;
mod error;
mod model;
mod wire;

pub use client::{Client, ClientBuilder, Credentials, Environment};
pub use error::{Error, Result, ServiceError};
pub use model::{
    MessageCode, MessageLevel, Response, ServiceKind, ServiceMessage, ServiceStatus, XmlDateTime,
};
