use crate::fir::{AreaCode, FirCode, LFFM_CODE};
use regex::Regex;
use std::sync::OnceLock;

/// Parse a GNG-style sector/profile filename and extract the FIR code and
/// AIRAC cycle if present.
///
/// Examples:
///   "LFBB-Bordeaux-260301-0003.sct"  → (LFBB, "2603")
///   "LFEE-Reims-260301-0003.ese"     → (LFEE, "2603")
///   "LFFM-Military_…-260801-0001.sct" → None (LFFM is an area, not a FIR —
///                                       use `parse_gng_sector_target`)
///   "garbage.sct"                    → None
///
/// AIRAC encoding: the 6-digit middle group is taken as `YYMMSS` where the
/// first 4 chars (YYMM) are the cycle identifier. If the GNG convention turns
/// out to use the full 6 digits instead, change `cycle_from_six_digits`.
pub fn parse_gng_sector_filename(name: &str) -> Option<(FirCode, Option<String>)> {
    // Look for a leading FIR code followed by a delimiter we recognise.
    let fir = leading_fir_code(name)?;
    Some((fir, parse_airac_cycle(name)))
}

/// The GNG "combined" sector code for the northern France pack. Its single
/// sector file serves both the Paris (LFFF) and Reims (LFEE) FIRs.
pub const LFXXN_CODE: &str = "LFXXN";

/// Parse a GNG sector/profile filename into the *set* of areas it covers plus
/// the AIRAC cycle. Unlike [`parse_gng_sector_filename`], a combined code such as
/// `LFXXN` resolves to several FIRs (e.g. `LFXXN-Paris-Reims_…` → LFFF + LFEE),
/// and the non-FIR `LFFM` area is recognised too. A regular `<AREA>-…` filename
/// resolves to that single area.
///
/// Examples:
///   "LFXXN-Paris-Reims_20260605153747-260501-0001.sct" → ([LFFF, LFEE], "2605")
///   "LFBB-Bordeaux-260301-0003.sct"                    → ([LFBB], "2603")
///   "LFFM-Military_20260807195057-260801-0001.sct"     → ([LFFM], "2608")
pub fn parse_gng_sector_target(name: &str) -> Option<(Vec<AreaCode>, Option<String>)> {
    // A combined code (e.g. LFXXN) takes precedence; no area code is a prefix of
    // it, so the order relative to the single-area check is not load-bearing.
    let areas = if let Some(combined) = leading_combined_code(name) {
        combined.iter().copied().map(AreaCode::Fir).collect()
    } else {
        vec![leading_area_code(name)?]
    };
    Some((areas, parse_airac_cycle(name)))
}

/// The single area a `<AREA>-…` / `<AREA>.…` filename targets: a FIR, or the
/// military/legacy `LFFM` area, which ships its own sector file just like a FIR.
pub fn leading_area_code(name: &str) -> Option<AreaCode> {
    if has_leading_code(name, LFFM_CODE) {
        return Some(AreaCode::Lffm);
    }
    leading_fir_code(name).map(AreaCode::Fir)
}

/// Extract the AIRAC cycle: the first 6-digit numeric group found between
/// dashes, of which the first 4 digits (YYMM) are the cycle. Returns `None` when
/// no such group is present (e.g. a bare `LFBB.sct`). The `_`-prefixed 14-digit
/// creation timestamp in newer GNG names is not delimited by dashes, so it is
/// never mistaken for the cycle.
fn parse_airac_cycle(name: &str) -> Option<String> {
    SIX_DIGIT_GROUP
        .get_or_init(|| Regex::new(r"-(\d{6})-").expect("regex"))
        .captures(name)
        .and_then(|c| c.get(1))
        .map(|m| cycle_from_six_digits(m.as_str()))
}

/// If `name` starts with a known combined code followed by a separator (or is
/// exactly the code), the FIRs it covers. Mirrors [`leading_fir_code`]'s
/// prefix+separator matching so that e.g. `LFXXNX` does not match. Accepts both
/// the GNG sector filename (`LFXXN-Paris-Reims_…`) and the bare package folder
/// segment (`LFXXN`).
pub fn leading_combined_code(name: &str) -> Option<&'static [FirCode]> {
    const COMBINED: &[(&str, &[FirCode])] =
        &[(LFXXN_CODE, &[FirCode::LFFF, FirCode::LFEE])];
    COMBINED
        .iter()
        .find(|(code, _)| has_leading_code(name, code))
        .map(|(_, firs)| *firs)
}

static SIX_DIGIT_GROUP: OnceLock<Regex> = OnceLock::new();

/// Whether `name` starts with `code` followed by either nothing, a separator
/// (`-`, `_`, ` `) or a dot (for `LFBB.sct`-style names). The separator check is
/// what keeps e.g. `LFXXNX-…` from matching `LFXXN`.
fn has_leading_code(name: &str, code: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    match upper.strip_prefix(code) {
        Some(rest) => rest.is_empty() || matches!(rest.chars().next(), Some('-' | '_' | ' ' | '.')),
        None => false,
    }
}

fn leading_fir_code(name: &str) -> Option<FirCode> {
    FirCode::ALL
        .into_iter()
        .find(|fir| has_leading_code(name, fir.as_str()))
}

fn cycle_from_six_digits(six: &str) -> String {
    // Take the first 4 digits as the AIRAC cycle code (YYMM).
    // The last two digits typically encode an AIRAC sub-revision.
    six[..4].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_gng_sector_filename() {
        let (fir, cycle) = parse_gng_sector_filename("LFBB-Bordeaux-260301-0003.sct").unwrap();
        assert_eq!(fir, FirCode::LFBB);
        assert_eq!(cycle.as_deref(), Some("2603"));
    }

    #[test]
    fn parses_ese_extension() {
        let (fir, _) = parse_gng_sector_filename("LFEE-Reims-260301-0003.ese").unwrap();
        assert_eq!(fir, FirCode::LFEE);
    }

    #[test]
    fn lffm_is_not_a_fir() {
        // The FIR-only parser must not claim LFFM; it is an area of its own and
        // is resolved by `parse_gng_sector_target`.
        assert!(
            parse_gng_sector_filename("LFFM-Military_20260807195057-260801-0001.sct").is_none()
        );
    }

    #[test]
    fn rejects_unrelated_filenames() {
        assert!(parse_gng_sector_filename("README.txt").is_none());
        assert!(parse_gng_sector_filename("garbage.sct").is_none());
        assert!(parse_gng_sector_filename("LFXX-Base.sct").is_none());
    }

    #[test]
    fn parses_bare_fir_filename_without_cycle() {
        let (fir, cycle) = parse_gng_sector_filename("LFBB.sct").unwrap();
        assert_eq!(fir, FirCode::LFBB);
        assert!(cycle.is_none());
    }

    #[test]
    fn case_insensitive_fir_match() {
        let (fir, _) = parse_gng_sector_filename("lfmm-Marseille-260301-0003.sct").unwrap();
        assert_eq!(fir, FirCode::LFMM);
    }

    #[test]
    fn does_not_match_lfxx_or_other_lf_codes_as_fir() {
        // LFXX is the shared pack code, not a FIR — should not parse as one.
        assert!(parse_gng_sector_filename("LFXX-Base-260301-0003.sct").is_none());
        // LFXY is gibberish — not a known FIR.
        assert!(parse_gng_sector_filename("LFXY-Random.sct").is_none());
    }

    #[test]
    fn combined_lfxxn_resolves_to_lfff_and_lfee() {
        // The real GNG combined name embeds an `_`-prefixed 14-digit creation
        // timestamp before the AIRAC group; the cycle must be the `-260501-`
        // group (→ 2605), not any 6 digits of the timestamp.
        let (areas, cycle) =
            parse_gng_sector_target("LFXXN-Paris-Reims_20260605153747-260501-0001.sct").unwrap();
        assert_eq!(areas, vec![AreaCode::Fir(FirCode::LFFF), AreaCode::Fir(FirCode::LFEE)]);
        assert_eq!(cycle.as_deref(), Some("2605"));
    }

    #[test]
    fn combined_lfxxn_ese_variant() {
        let (areas, cycle) =
            parse_gng_sector_target("LFXXN-Paris-Reims_20260605153747-260501-0001.ese").unwrap();
        assert_eq!(areas, vec![AreaCode::Fir(FirCode::LFFF), AreaCode::Fir(FirCode::LFEE)]);
        assert_eq!(cycle.as_deref(), Some("2605"));
    }

    #[test]
    fn combined_lfxxn_is_not_a_single_fir() {
        // The legacy single-FIR parser must not claim LFXXN as a FIR.
        assert!(parse_gng_sector_filename("LFXXN-Paris-Reims_20260605153747-260501-0001.sct")
            .is_none());
    }

    #[test]
    fn sector_target_falls_back_to_single_fir() {
        let (areas, cycle) =
            parse_gng_sector_target("LFBB-Bordeaux-260301-0003.sct").unwrap();
        assert_eq!(areas, vec![AreaCode::Fir(FirCode::LFBB)]);
        assert_eq!(cycle.as_deref(), Some("2603"));
    }

    #[test]
    fn lffm_is_a_sector_target_of_its_own() {
        // LFFM is not an ICAO FIR, but it owns `LFXX/Sectors/LFFM.sct` just like
        // a FIR owns its own — `CoFrance LFFM.prf` reads it from there. Real GNG
        // name, with the `_`-prefixed 14-digit creation timestamp before the
        // `-260801-` AIRAC group.
        let (areas, cycle) =
            parse_gng_sector_target("LFFM-Military_20260807195057-260801-0001.sct").unwrap();
        assert_eq!(areas, vec![AreaCode::Lffm]);
        assert_eq!(cycle.as_deref(), Some("2608"));
    }

    #[test]
    fn lffm_ese_variant() {
        let (areas, cycle) =
            parse_gng_sector_target("LFFM-Military_20260807195057-260801-0001.ese").unwrap();
        assert_eq!(areas, vec![AreaCode::Lffm]);
        assert_eq!(cycle.as_deref(), Some("2608"));
    }

    #[test]
    fn lffm_area_requires_separator() {
        assert!(parse_gng_sector_target("LFFMX-Random.sct").is_none());
    }

    #[test]
    fn combined_code_requires_separator() {
        // A longer code that merely starts with LFXXN must not match.
        assert!(parse_gng_sector_target("LFXXNX-Random.sct").is_none());
    }
}
