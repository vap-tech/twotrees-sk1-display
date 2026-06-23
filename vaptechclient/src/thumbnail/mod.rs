pub mod cache;
pub mod decoder;
pub mod tjc_encoder;
pub mod worker;

use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    pub file_path: String,
    pub target: ThumbnailTarget,
    pub width: u16,
    pub height: u16,
    pub encoder_version: u8,
}

impl ThumbnailKey {
    pub fn print(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            target: ThumbnailTarget::PrintPage,
            width: 155,
            height: 155,
            encoder_version: tjc_encoder::ENCODER_VERSION,
        }
    }

    pub fn file_slot(file_path: impl Into<String>, slot: u8) -> Self {
        Self {
            file_path: file_path.into(),
            target: ThumbnailTarget::FileSlot { slot },
            width: 155,
            height: 155,
            encoder_version: tjc_encoder::ENCODER_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThumbnailTarget {
    PrintPage,
    FileSlot { slot: u8 },
    PreviewPage,
    ResultPage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailSource {
    GcodeFile(String),
    PreparedChunks(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailRequest {
    Prepare {
        key: ThumbnailKey,
        source: ThumbnailSource,
    },
}

impl ThumbnailRequest {
    pub fn key(&self) -> &ThumbnailKey {
        match self {
            Self::Prepare { key, .. } => key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailResult {
    pub key: ThumbnailKey,
    pub result: Result<Vec<crate::hmi::command::HmiCommand>, String>,
}
