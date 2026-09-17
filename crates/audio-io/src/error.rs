//! Error types (placeholder for Task 3)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioIoError {}

pub type Result<T> = std::result::Result<T, AudioIoError>;
