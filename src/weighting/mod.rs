use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Weighting or scaling strategy to apply in a regularization experiment.
///
/// These strategies are intentionally simple in the first `0.4` slice so the
/// surrounding experiment machinery can stabilize before we thread them through
/// the full solver pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeightingStrategy {
    None,
    Q,
    QSquared,
    QPower { alpha: f64 },
}

impl WeightingStrategy {
    /// Return the multiplicative scale associated with the strategy at one `q`.
    pub fn scale(self, q: f64) -> Result<f64, WeightingError> {
        if !q.is_finite() || q < 0.0 {
            return Err(WeightingError::InvalidQ { q });
        }

        let scale = match self {
            Self::None => 1.0,
            Self::Q => q,
            Self::QSquared => q * q,
            Self::QPower { alpha } => q.powf(alpha),
        };

        if !scale.is_finite() {
            return Err(WeightingError::NonFiniteScale { q });
        }

        Ok(scale)
    }

    /// Apply a residual-weighting transform to one observation.
    ///
    /// The returned `scale` is applied to both the design-matrix row and the
    /// observed intensity so the residual itself is reweighted. The statistical
    /// uncertainty stays on the original scale, which means the final objective
    /// becomes `||W S (A c - y)||^2` rather than a no-op similarity transform.
    pub fn transform_observation(
        self,
        q: f64,
        intensity: f64,
        sigma: f64,
    ) -> Result<WeightedObservation, WeightingError> {
        if !intensity.is_finite() || !sigma.is_finite() || sigma <= 0.0 {
            return Err(WeightingError::InvalidObservation { intensity, sigma });
        }

        let scale = self.scale(q)?;
        Ok(WeightedObservation {
            scale,
            intensity: scale * intensity,
            sigma,
        })
    }

    pub fn as_config_string(self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::Q => "q".to_string(),
            Self::QSquared => "q2".to_string(),
            Self::QPower { alpha } => format!("q^{alpha}"),
        }
    }
}

impl FromStr for WeightingStrategy {
    type Err = WeightingParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "none" => Ok(Self::None),
            "q" => Ok(Self::Q),
            "q2" | "q^2" => Ok(Self::QSquared),
            other => {
                let Some(alpha) = other.strip_prefix("q^") else {
                    return Err(WeightingParseError {
                        value: other.to_string(),
                    });
                };
                let Ok(alpha) = alpha.parse::<f64>() else {
                    return Err(WeightingParseError {
                        value: other.to_string(),
                    });
                };
                if !alpha.is_finite() {
                    return Err(WeightingParseError {
                        value: other.to_string(),
                    });
                }
                Ok(Self::QPower { alpha })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedObservation {
    pub scale: f64,
    pub intensity: f64,
    pub sigma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeightingParseError {
    value: String,
}

impl fmt::Display for WeightingParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported weighting strategy `{}`; expected one of `none`, `q`, `q2`, or `q^<alpha>`",
            self.value
        )
    }
}

impl Error for WeightingParseError {}

#[derive(Debug, Clone, PartialEq)]
pub enum WeightingError {
    InvalidQ { q: f64 },
    InvalidObservation { intensity: f64, sigma: f64 },
    NonFiniteScale { q: f64 },
}

impl fmt::Display for WeightingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQ { q } => write!(f, "q must be finite and non-negative, but was {q}"),
            Self::InvalidObservation { intensity, sigma } => write!(
                f,
                "intensity must be finite and sigma must be finite and positive, but found intensity={intensity}, sigma={sigma}"
            ),
            Self::NonFiniteScale { q } => {
                write!(f, "weighting scale became non-finite at q={q}")
            }
        }
    }
}

impl Error for WeightingError {}

#[cfg(test)]
mod tests {
    use super::{WeightedObservation, WeightingStrategy};
    use std::str::FromStr;

    #[test]
    fn parses_builtin_weighting_strategies() {
        assert_eq!(
            WeightingStrategy::from_str("none").unwrap(),
            WeightingStrategy::None
        );
        assert_eq!(
            WeightingStrategy::from_str("q").unwrap(),
            WeightingStrategy::Q
        );
        assert_eq!(
            WeightingStrategy::from_str("q2").unwrap(),
            WeightingStrategy::QSquared
        );
        assert_eq!(
            WeightingStrategy::from_str("q^1.5").unwrap(),
            WeightingStrategy::QPower { alpha: 1.5 }
        );
    }

    #[test]
    fn rejects_unknown_weighting_strategies() {
        assert!(WeightingStrategy::from_str("banana").is_err());
    }

    #[test]
    fn transforms_observation_into_residual_weighting_form() {
        let transformed = WeightingStrategy::QSquared
            .transform_observation(2.0, 3.0, 0.5)
            .unwrap();

        assert_eq!(
            transformed,
            WeightedObservation {
                scale: 4.0,
                intensity: 12.0,
                sigma: 0.5,
            }
        );
    }
}
