use thiserror::Error;

use super::{EngineId, ParameterId};
use crate::protocol::ProtocolError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("SynthPilot doesn't know the parameter {0}.")]
    UnknownParameter(ParameterId),
    #[error("{value} is outside the range of {parameter} ({min} to {max}).")]
    OutOfRange {
        parameter: ParameterId,
        value: i32,
        min: i32,
        max: i32,
    },
    #[error("Part {part} doesn't exist. Choose a Part from 1 to {max}.")]
    PartOutOfRange { part: u8, max: u8 },
    #[error("There's no Element or Operator {element}. Choose one from 1 to {max}.")]
    ElementOutOfRange { element: u8, max: u8 },
    #[error("Choose an Element or Operator for {0}.")]
    MissingElement(ParameterId),
    #[error("{0} applies to the whole Part, not to one Element or Operator.")]
    UnexpectedElement(ParameterId),
    #[error("{0} applies to the whole Performance, not to one Part.")]
    PerformanceOnly(ParameterId),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    #[error(transparent)]
    Invalid(#[from] ValidationError),
    #[error("{0} can't be edited in SynthPilot yet.")]
    NotImplemented(ParameterId),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("Part {0} is empty, so there's nothing to edit.")]
    EmptyPart(u8),
    #[error("This keyboard has no settings shared by the whole Performance.")]
    NoPerformance,
    #[error(
        "Part {part} is a {actual} Part, not {expected}. Select Read from keyboard to update it."
    )]
    EngineMismatch {
        part: u8,
        expected: EngineId,
        actual: EngineId,
    },
    #[error(transparent)]
    Invalid(#[from] ValidationError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}
