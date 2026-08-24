use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FirCode {
    LFBB,
    LFEE,
    LFFF,
    LFMM,
    LFRR,
}

impl FirCode {
    pub const ALL: [FirCode; 5] = [
        FirCode::LFBB,
        FirCode::LFEE,
        FirCode::LFFF,
        FirCode::LFMM,
        FirCode::LFRR,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FirCode::LFBB => "LFBB",
            FirCode::LFEE => "LFEE",
            FirCode::LFFF => "LFFF",
            FirCode::LFMM => "LFMM",
            FirCode::LFRR => "LFRR",
        }
    }
}

impl FromStr for FirCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "LFBB" => Ok(FirCode::LFBB),
            "LFEE" => Ok(FirCode::LFEE),
            "LFFF" => Ok(FirCode::LFFF),
            "LFMM" => Ok(FirCode::LFMM),
            "LFRR" => Ok(FirCode::LFRR),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for FirCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The military / legacy area. Not an ICAO FIR, but it owns an area folder and a
/// sector file (`LFXX/Sectors/LFFM.sct`) exactly like a FIR does.
pub const LFFM_CODE: &str = "LFFM";

/// An installable area: one of the French FIRs, or `LFFM`.
///
/// `LFFM` is deliberately kept out of [`FirCode`] — it is not a FIR and the
/// GitHub overlay gates it separately (it is only installed when its package is
/// selected). It is, however, laid out like one: its own folder, its own `.prf`
/// reading `\ICAO\…` / `\NavData\…` relative to that folder, and its own sector
/// file. Anything keyed on "which area does this file belong to" therefore uses
/// this type rather than `FirCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AreaCode {
    Fir(FirCode),
    Lffm,
}

impl AreaCode {
    /// Every area, in a stable order.
    pub const ALL: [AreaCode; 6] = [
        AreaCode::Fir(FirCode::LFBB),
        AreaCode::Fir(FirCode::LFEE),
        AreaCode::Fir(FirCode::LFFF),
        AreaCode::Fir(FirCode::LFMM),
        AreaCode::Fir(FirCode::LFRR),
        AreaCode::Lffm,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AreaCode::Fir(fir) => fir.as_str(),
            AreaCode::Lffm => LFFM_CODE,
        }
    }
}

impl From<FirCode> for AreaCode {
    fn from(fir: FirCode) -> Self {
        AreaCode::Fir(fir)
    }
}

impl FromStr for AreaCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case(LFFM_CODE) {
            return Ok(AreaCode::Lffm);
        }
        s.parse::<FirCode>().map(AreaCode::Fir)
    }
}

impl std::fmt::Display for AreaCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
