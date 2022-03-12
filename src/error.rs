use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiddlewareError {
    #[error("Unable to create Participant")]
    DDSError(#[from] cyclonedds_rs::DDSError),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader {
        expected: String,
        found: String,
    },
    #[error("Data structures inconsistent (would have panicked)")]
    InconsistentDataStructure,
}


#[derive(Error, Debug)]
pub enum DDSError {
    #[error("DDS Error")]
    DdsError,
    #[error("DDS Unsupported")]
    Unsupported,
    #[error("Bad Parameter")]
    BadParameter,
    #[error("Preconditions not met")]
    PreconditionNotMet,
    #[error("Out of Resources")]
    OutOfResources,
    #[error("Not Enabled")]
    NotEnabled,
    #[error("Immutable Policy")]
    ImmutablePolicy,
    #[error("Inconsistent Policy")]
    InconsistentPolicy,
    #[error("Already Deleted")]
    AlreadyDeleted,
    #[error("Timeout")]
    Timeout,
    #[error("No Data")]
    NoData,
    #[error("Illegal Operation")]
    IllegalOperation,
    #[error("Not allowed by security")]
    NotAllowedBySecurity,
}

impl From<cyclonedds_rs::DDSError> for DDSError {
    fn from(cdds_error: cyclonedds_rs::DDSError) -> Self {
        match cdds_error {
            cyclonedds_rs::DDSError::DdsOk => panic!("Don't expect a non-error"),
            cyclonedds_rs::DDSError::DdsError => Self::DdsError,
            cyclonedds_rs::DDSError::Unsupported => Self::Unsupported,
            cyclonedds_rs::DDSError::BadParameter => Self::BadParameter,
            cyclonedds_rs::DDSError::PreconditionNotMet => Self::PreconditionNotMet,
            cyclonedds_rs::DDSError::OutOfResources => Self::OutOfResources,
            cyclonedds_rs::DDSError::NotEnabled => Self::NotEnabled,
            cyclonedds_rs::DDSError::ImmutablePolicy => Self::ImmutablePolicy,
            cyclonedds_rs::DDSError::InconsistentPolicy => Self::InconsistentPolicy,
            cyclonedds_rs::DDSError::AlreadyDeleted => Self::AlreadyDeleted,
            cyclonedds_rs::DDSError::Timeout => Self::Timeout,
            cyclonedds_rs::DDSError::NoData => Self::NoData,
            cyclonedds_rs::DDSError::IllegalOperation => Self::IllegalOperation,
            cyclonedds_rs::DDSError::NotAllowedBySecurity => Self::NotAllowedBySecurity,
        }
    }
}