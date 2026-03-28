use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

/// One sampled point from a reference `P(r)` curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrPoint {
    pub r: f64,
    pub p_of_r: f64,
    pub error: f64,
}

/// A validated sampled `P(r)` distribution from a reference `.out` file.
#[derive(Debug, Clone, PartialEq)]
pub struct PrDistribution {
    points: Vec<PrPoint>,
}

impl PrDistribution {
    pub fn new(points: Vec<PrPoint>) -> Result<Self, ParseReferencePrError> {
        if points.is_empty() {
            return Err(ParseReferencePrError::NoDataRows);
        }

        for point in &points {
            if !point.r.is_finite() || !point.p_of_r.is_finite() || !point.error.is_finite() {
                return Err(ParseReferencePrError::NonFiniteValue);
            }
        }

        for window in points.windows(2) {
            if window[0].r >= window[1].r {
                return Err(ParseReferencePrError::NonMonotonicR {
                    previous_r: window[0].r,
                    current_r: window[1].r,
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

    pub fn points(&self) -> &[PrPoint] {
        &self.points
    }
}

#[derive(Debug)]
pub enum ParseReferencePrError {
    Io(std::io::Error),
    MissingHeader,
    NoDataRows,
    NonFiniteValue,
    NonMonotonicR { previous_r: f64, current_r: f64 },
}

impl fmt::Display for ParseReferencePrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read reference P(r) file: {error}"),
            Self::MissingHeader => write!(f, "could not find the `R P(R) ERROR` block header"),
            Self::NoDataRows => write!(f, "no reference P(r) rows were found after the header"),
            Self::NonFiniteValue => write!(f, "encountered a non-finite r, P(r), or error value"),
            Self::NonMonotonicR {
                previous_r,
                current_r,
            } => write!(
                f,
                "r values must be strictly increasing, but found {previous_r} followed by {current_r}"
            ),
        }
    }
}

impl Error for ParseReferencePrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::MissingHeader
            | Self::NoDataRows
            | Self::NonFiniteValue
            | Self::NonMonotonicR { .. } => None,
        }
    }
}

impl From<std::io::Error> for ParseReferencePrError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn parse_reference_pr_file(
    path: impl AsRef<Path>,
) -> Result<PrDistribution, ParseReferencePrError> {
    let contents = fs::read_to_string(path)?;
    parse_reference_pr(&contents)
}

/// Parse the `P(r)` block from a GNOM-style `.out` file.
///
/// The parser:
///
/// 1. Scans for the `R P(R) ERROR` header
/// 2. Starts collecting rows after that header
/// 3. Accepts only lines with exactly three numeric columns
/// 4. Requires the first column (`r`) to increase strictly
/// 5. Stops as soon as the contiguous data block ends
///
/// This lets us ignore the rest of the `.out` file, including earlier numeric
/// sections and any footer content in older files.
pub fn parse_reference_pr(contents: &str) -> Result<PrDistribution, ParseReferencePrError> {
    let mut seen_header = false;
    let mut points = Vec::new();
    let mut previous_r: Option<f64> = None;

    for line in contents.lines() {
        if !seen_header {
            if is_reference_pr_header(line) {
                seen_header = true;
            }
            continue;
        }

        let Some(point) = parse_pr_point_line(line) else {
            if points.is_empty() {
                continue;
            }
            break;
        };

        if let Some(previous) = previous_r
            && point.r <= previous
        {
            break;
        }

        previous_r = Some(point.r);
        points.push(point);
    }

    if !seen_header {
        return Err(ParseReferencePrError::MissingHeader);
    }

    PrDistribution::new(points)
}

fn is_reference_pr_header(line: &str) -> bool {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    fields == ["R", "P(R)", "ERROR"]
}

fn parse_pr_point_line(line: &str) -> Option<PrPoint> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut fields = trimmed.split_ascii_whitespace();
    let r = fields.next()?.parse::<f64>().ok()?;
    let p_of_r = fields.next()?.parse::<f64>().ok()?;
    let error = fields.next()?.parse::<f64>().ok()?;

    if fields.next().is_some() {
        return None;
    }

    Some(PrPoint { r, p_of_r, error })
}

#[cfg(test)]
mod tests {
    use super::{ParseReferencePrError, parse_reference_pr, parse_reference_pr_file};
    use std::path::PathBuf;

    fn reference_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("reference")
            .join(filename)
    }

    #[test]
    fn parses_reference_pr_block_from_real_file() {
        let distribution = parse_reference_pr_file(reference_path("SASDME2.out")).unwrap();

        assert!(!distribution.is_empty());
        assert_eq!(distribution.points()[0].r, 0.0);
        assert_eq!(distribution.points()[0].p_of_r, 0.0);
        assert_eq!(distribution.points()[0].error, 0.0);
    }

    #[test]
    fn parses_reference_file_with_crlf_line_endings() {
        let distribution = parse_reference_pr_file(reference_path("SASDYU3.out")).unwrap();

        assert!(!distribution.is_empty());
        assert!(distribution.len() > 100);
        assert_eq!(distribution.points()[0].r, 0.0);
    }

    #[test]
    fn stops_at_footer_or_non_data_after_the_block() {
        let input = "\
Some preamble
R          P(R)      ERROR

0.0 0.0 0.0
1.0 2.0 0.1
2.0 3.0 0.2
Footer text
3.0 4.0 0.3
";

        let distribution = parse_reference_pr(input).unwrap();

        assert_eq!(distribution.len(), 3);
        assert_eq!(distribution.points()[2].r, 2.0);
    }

    #[test]
    fn stops_when_r_is_no_longer_increasing() {
        let input = "\
R P(R) ERROR
0.0 0.0 0.0
1.0 2.0 0.1
0.5 3.0 0.2
";

        let distribution = parse_reference_pr(input).unwrap();

        assert_eq!(distribution.len(), 2);
        assert_eq!(distribution.points()[1].r, 1.0);
    }

    #[test]
    fn rejects_missing_header() {
        let error = parse_reference_pr("0.0 0.0 0.0").unwrap_err();
        assert!(matches!(error, ParseReferencePrError::MissingHeader));
    }

    #[test]
    fn rejects_header_without_data_block() {
        let error = parse_reference_pr("R P(R) ERROR\nFooter").unwrap_err();
        assert!(matches!(error, ParseReferencePrError::NoDataRows));
    }
}
