use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditMode {
    Advisory,
    Guarded,
    Standard,
    Ratchet,
    Release,
}

impl AuditMode {
    pub fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "advisory" => Self::Advisory,
            "guarded" => Self::Guarded,
            "standard" => Self::Standard,
            "ratchet" => Self::Ratchet,
            "release" => Self::Release,
            other => bail!("unknown audit mode `{other}`; expected advisory, guarded, standard, ratchet, or release"),
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Guarded => "guarded",
            Self::Standard => "standard",
            Self::Ratchet => "ratchet",
            Self::Release => "release",
        }
    }
}
