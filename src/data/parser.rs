use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

/// One measured SAXS point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SaxsPoint {
    pub q: f64,
    pub intensity: f64,
    pub sigma: f64,
}

/// A validated SAXS curve.
#[derive(Debug, Clone, PartialEq)]
pub struct SaxsCurve {
    points: Vec<SaxsPoint>,
}

impl SaxsCurve {
    /// Build a validated curve from already-parsed points.
    pub fn new(points: Vec<SaxsPoint>) -> Result<Self, ParseCurveError> {
        if points.is_empty() {
            return Err(ParseCurveError::NoDataRows);
        }

        for point in &points {
            if !point.q.is_finite() || !point.intensity.is_finite() || !point.sigma.is_finite() {
                return Err(ParseCurveError::NonFiniteValue);
            }

            if point.sigma <= 0.0 {
                return Err(ParseCurveError::NonPositiveSigma { sigma: point.sigma });
            }
        }

        for window in points.windows(2) {
            if window[0].q >= window[1].q {
                return Err(ParseCurveError::NonMonotonicQ {
                    previous_q: window[0].q,
                    current_q: window[1].q,
                });
            }
        }

        Ok(Self { points })
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn points(&self) -> &[SaxsPoint] {
        &self.points
    }
}

#[derive(Debug)]
pub enum ParseCurveError {
    Io(std::io::Error),
    NoDataRows,
    NonFiniteValue,
    NonPositiveSigma { sigma: f64 },
    NonMonotonicQ { previous_q: f64, current_q: f64 },
}

impl fmt::Display for ParseCurveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read SAXS data file: {error}"),
            Self::NoDataRows => write!(f, "no numeric SAXS data rows were found"),
            Self::NonFiniteValue => {
                write!(f, "encountered a non-finite q, intensity, or sigma value")
            }
            Self::NonPositiveSigma { sigma } => {
                write!(f, "encountered a non-positive sigma value: {sigma}")
            }
            Self::NonMonotonicQ {
                previous_q,
                current_q,
            } => write!(
                f,
                "q values must be strictly increasing, but found {previous_q} followed by {current_q}"
            ),
        }
    }
}

impl Error for ParseCurveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NoDataRows
            | Self::NonFiniteValue
            | Self::NonPositiveSigma { .. }
            | Self::NonMonotonicQ { .. } => None,
        }
    }
}

impl From<std::io::Error> for ParseCurveError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Parse a SAXS curve from a text file.
pub fn parse_ascii_curve_file(path: impl AsRef<Path>) -> Result<SaxsCurve, ParseCurveError> {
    let contents = fs::read_to_string(path)?;
    parse_ascii_curve(&contents)
}

/// Parse a SAXS curve from ASCII text.
///
/// Any line with exactly three whitespace-separated fields that all parse as
/// `f64` is treated as a data row. All other lines are ignored. This keeps the
/// parser flexible across common SAXS text formats while still enforcing strict
/// validation once the numeric rows have been collected.
pub fn parse_ascii_curve(contents: &str) -> Result<SaxsCurve, ParseCurveError> {
    let points = contents
        .lines()
        .filter_map(parse_point_line)
        .collect::<Vec<_>>();

    SaxsCurve::new(points)
}

fn parse_point_line(line: &str) -> Option<SaxsPoint> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut fields = trimmed.split_ascii_whitespace();
    let q = fields.next()?.parse::<f64>().ok()?;
    let intensity = fields.next()?.parse::<f64>().ok()?;
    let sigma = fields.next()?.parse::<f64>().ok()?;

    if fields.next().is_some() {
        return None;
    }

    Some(SaxsPoint {
        q,
        intensity,
        sigma,
    })
}

#[cfg(test)]
mod tests {
    use super::{ParseCurveError, parse_ascii_curve, parse_ascii_curve_file};
    use std::path::PathBuf;

    fn example_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("examples")
            .join(filename)
    }

    #[test]
    fn parses_atsas_style_example_file() {
        let curve = parse_ascii_curve_file(example_path("SASDME2.dat")).unwrap();

        assert!(!curve.is_empty());
        assert!(curve.len() > 100);
        assert_eq!(curve.points()[0].q, 8.0e-3);
        assert_eq!(curve.points()[0].intensity, 8.650406e1);
        assert_eq!(curve.points()[0].sigma, 4.69173);
    }

    #[test]
    fn parses_hash_comment_style_example_file() {
        let curve = parse_ascii_curve_file(example_path("SASDYU3.dat")).unwrap();

        assert!(!curve.is_empty());
        assert!(curve.len() > 100);
        assert_eq!(curve.points()[0].q, 2.90283960e-3);
        assert_eq!(curve.points()[0].intensity, 1.52516586);
        assert_eq!(curve.points()[0].sigma, 1.09645689e-1);
    }

    #[test]
    fn ignores_non_numeric_lines_and_accepts_numeric_rows() {
        let input = "\
# comment
Header line
  1.0  2.0  0.1
still not data
2.0 3.0 0.2
";

        let curve = parse_ascii_curve(input).unwrap();

        assert_eq!(curve.len(), 2);
        assert_eq!(curve.points()[1].q, 2.0);
        assert_eq!(curve.points()[1].intensity, 3.0);
        assert_eq!(curve.points()[1].sigma, 0.2);
    }

    #[test]
    fn rejects_files_without_data_rows() {
        let error = parse_ascii_curve("# only comments\nheader").unwrap_err();
        assert!(matches!(error, ParseCurveError::NoDataRows));
    }

    #[test]
    fn rejects_non_monotonic_q_values() {
        let input = "\
1.0 2.0 0.1
0.9 1.5 0.1
";

        let error = parse_ascii_curve(input).unwrap_err();
        assert!(matches!(error, ParseCurveError::NonMonotonicQ { .. }));
    }

    #[test]
    fn rejects_non_positive_sigma_values() {
        let input = "1.0 2.0 0.0";

        let error = parse_ascii_curve(input).unwrap_err();
        assert!(matches!(error, ParseCurveError::NonPositiveSigma { .. }));
    }

    #[test]
    fn rejects_non_finite_values() {
        let error = parse_ascii_curve("1.0 NaN 0.1").unwrap_err();
        assert!(matches!(error, ParseCurveError::NonFiniteValue));
    }

    #[test]
    fn returns_io_error_for_missing_file() {
        let missing = example_path("definitely_missing.dat");
        let error = parse_ascii_curve_file(missing).unwrap_err();
        assert!(matches!(error, ParseCurveError::Io(_)));
    }
}
