mod frame;
pub mod registrar;
mod tracing_processor;
pub mod weather;

pub use frame::WeatherFrame;

use crate::errors::WeatherError;
use chrono::{DateTime, Utc};
use disintegrate::{IdentifierType, IdentifierValue, IntoIdentifierValue};
use geojson::Feature;
use rust_decimal::prelude::*;
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use strum_macros::{Display, EnumMessage, EnumString, IntoStaticStr, VariantNames};
use url::Url;

pub fn transpose_result<T, E>(
    results: impl IntoIterator<Item = Result<T, E>>,
) -> Result<Vec<T>, E> {
    let mut acc = Vec::new();

    for value in results {
        acc.push(value?);
    }

    Ok(acc)
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    IntoParams,
    sqlx::Type,
    ToSchema,
    bitcode::Encode,
    bitcode::Decode,
    Serialize,
    Deserialize,
)]
#[into_params(names("zones_code"))]
#[repr(transparent)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct LocationZoneCode(String);

impl fmt::Display for LocationZoneCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl LocationZoneCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn from_url(url: impl Into<Url>) -> Result<(Option<LocationZoneType>, Self), WeatherError> {
        let url = url.into();
        url.path_segments()
            .and_then(|segments| {
                let segments: Vec<_> = segments.collect();
                let nr_segments = segments.len();

                if nr_segments < 2 {
                    None
                } else {
                    let zone_code = LocationZoneCode::new(segments[nr_segments - 1]);
                    let zone_type = LocationZoneType::from_str(segments[nr_segments - 2]).ok();
                    Some((zone_type, zone_code))
                }
            })
            .ok_or_else(|| WeatherError::UrlNotZoneIdentifier(url))
    }

    pub fn parse(code_rep: impl Into<String>) -> Result<Self, WeatherError> {
        let code_rep = code_rep.into();
        let code = if code_rep.starts_with("http") {
            Self::from_url(Url::parse(&code_rep)?)?.1
        } else {
            Self::new(code_rep)
        };
        Ok(code)
    }
}

// impl<'q, DB> sqlx::Encode<'q, DB> for LocationZoneCode
// where
//     DB: sqlx::Database,
// {
//     fn encode_by_ref(&self, buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer) -> sqlx::encode::IsNull {
//         buf.put_str_lenenc(self.as_str());
//         sqlx::encode::IsNull::No
//     }
// }

impl IntoIdentifierValue for LocationZoneCode {
    const TYPE: IdentifierType = IdentifierType::String;

    fn into_identifier_value(self) -> IdentifierValue {
        IdentifierValue::String(self.0)
    }
}

impl From<LocationZoneCode> for String {
    fn from(code: LocationZoneCode) -> Self {
        code.0
    }
}

impl FromStr for LocationZoneCode {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl From<&str> for LocationZoneCode {
    fn from(zone_code: &str) -> Self {
        LocationZoneCode::new(zone_code)
    }
}

impl From<String> for LocationZoneCode {
    fn from(zone_code: String) -> Self {
        LocationZoneCode::new(zone_code)
    }
}

impl AsRef<str> for LocationZoneCode {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl core::ops::Deref for LocationZoneCode {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::borrow::Borrow<str> for LocationZoneCode {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    IntoStaticStr,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
pub enum LocationZoneType {
    Public,
    County,
    Forecast,
}

#[derive(Debug, PartialEq, Eq, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantitativeValue {
    pub value: Decimal,
    pub max_value: Decimal,
    pub min_value: Decimal,
    pub unit_code: Cow<'static, str>,
    pub quality_control: QualityControl,
}

impl QuantitativeValue {
    pub fn new(
        value: f32, min_value: f32, max_value: f32, unit_code: &'static str,
        quality_control: QualityControl,
    ) -> Self {
        Self {
            value: Decimal::from_f32(value).expect("infinite value"),
            max_value: Decimal::from_f32(max_value).expect("infinite value"),
            min_value: Decimal::from_f32(min_value).expect("infinite value"),
            unit_code: Cow::Borrowed(unit_code),
            quality_control,
        }
    }

    pub fn unit_code(&self) -> &str {
        self.unit_code.borrow()
    }
}

#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    VariantNames,
    EnumMessage,
    // EnumProperty,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "UPPERCASE")]
pub enum QualityControl {
    #[strum(message = "Verified, passed levels 1, 2, and 3")]
    V,

    #[strum(message = "Subjective good")]
    G,

    #[strum(message = "Screened, passed levels 1 and 2")]
    S,

    #[strum(message = "Coarse pass, passed level 1")]
    C,

    #[strum(message = "Preliminary, no QC")]
    Z,

    #[strum(
        message = "Questioned, passed level 1, failed 2 or 3 where: level 1 = validity; level 2 = internal consistency, temporal consistency, statistical spatial consistency checks; level 3 = spatial consistency check"
    )]
    Q,

    #[strum(
        message = "Virtual temperature could not be calculated, air temperature passing all QC checks has been returned"
    )]
    T,

    #[strum(message = "Subjective bad")]
    B,

    #[strum(message = "Rejected/erroneous, failed level 1")]
    X,
}

impl QualityControl {
    pub fn level(&self) -> usize {
        match self {
            Self::V => 9,
            Self::G => 8,
            Self::S => 7,
            Self::C => 6,
            Self::Z => 5,
            Self::Q => 4,
            Self::T => 3,
            Self::B => 2,
            Self::X => 1,
        }
    }
}

impl PartialOrd for QualityControl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QualityControl {
    fn cmp(&self, other: &Self) -> Ordering {
        self.level().cmp(&other.level())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct ZoneForecast {
    // #[serde(deserialize_with = "ZoneForecast::deserialize_zone_from_url")]
    pub zone_code: String,

    pub updated: DateTime<Utc>,

    pub periods: Vec<ForecastDetail>,
}

impl TryFrom<Feature> for ZoneForecast {
    type Error = WeatherError;

    fn try_from(feature: Feature) -> Result<Self, Self::Error> {
        let zone_code = feature
            .property("zone")
            .and_then(|p| p.as_str())
            .ok_or_else(|| Self::Error::MissingFeature("zone".to_string()))?
            .to_string();

        let updated = Utc::now();

        let periods: Vec<Result<ForecastDetail, Self::Error>> = feature
            .property("periods")
            .and_then(|p| p.as_array())
            .cloned()
            .map(|ps| {
                ps.into_iter()
                    .map(|detail| serde_json::from_value(detail).map_err(|err| err.into()))
                    .collect()
            })
            .ok_or_else(|| Self::Error::MissingFeature("periods".to_string()))?;

        let nr_periods = periods.len();
        let periods: Vec<ForecastDetail> =
            periods.into_iter().try_fold(Vec::with_capacity(nr_periods), |acc, res| {
                match (acc, res) {
                    (mut acc_0, Ok(p)) => {
                        acc_0.push(p);
                        Ok(acc_0)
                    },

                    (_, Err(err)) => Err(err),
                }
            })?;

        Ok(Self { zone_code, updated, periods })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastDetail {
    pub name: String,

    #[serde(alias = "detailedForecast")]
    pub forecast: String,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherAlert {
    pub affected_zones: Vec<LocationZoneCode>,
    pub status: AlertStatus,
    pub message_type: AlertMessageType,

    /// The time of the origination of the alert message.
    pub sent: DateTime<Utc>,

    /// The effective time of the information of the alert message.
    pub effective: DateTime<Utc>,

    /// The expected time of the beginning of the subject event of the alert message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onset: Option<DateTime<Utc>>,

    /// The expiry time of the information of the alert message.
    pub expires: DateTime<Utc>,

    /// The expected end time of the subject event of the alert message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends: Option<DateTime<Utc>>,

    /// The code denoting the category of the subject event of the alert message.
    pub category: AlertCategory,
    pub severity: AlertSeverity,
    pub certainty: AlertCertainty,
    pub urgency: AlertUrgency,

    /// The text denoting the type of the subject event of the alert message.
    pub event: String,

    pub headline: Option<String>,

    /// An object representing a public alert message. Unless otherwise noted, the fields in this
    /// object correspond to the National Weather Service CAP v1.2 specification, which extends the
    /// OASIS Common Alerting Protocol (CAP) v1.2 specification and USA Integrated Public Alert and
    /// Warning System (IPAWS) Profile v1.0. Refer to this documentation for more complete
    /// information.
    /// http://docs.oasis-open.org/emergency/cap/v1.2/CAP-v1.2-os.html http://docs.oasis-open.org/emergency/cap/v1.2/ipaws-profile/v1.0/cs01/cap-v1.2-ipaws-profile-cs01.html https://alerts.weather.gov/#technical-notes-v12
    pub description: String,

    /// The text describing the recommended action to be taken by recipients of the alert message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,

    /// The code denoting the type of action recommended for the target audience. This corresponds
    /// to responseType in the CAP specification.
    pub response: AlertResponse,
}

impl TryFrom<Feature> for WeatherAlert {
    type Error = WeatherError;

    #[instrument(
        level = "trace",
        name = "DMR_WEATHER_ALERT_TRY_FROM_FEATURE",
        skip(f),
        err
    )]
    fn try_from(f: Feature) -> Result<Self, Self::Error> {
        let extract = PropertyExtractor::new("weather_alert", &f);

        let affected: Vec<String> = extract.property("affectedZones")?;
        let mut affected_zones = Vec::with_capacity(affected.len());
        for zone in affected {
            affected_zones.push(LocationZoneCode::parse(zone)?);
        }
        debug!("DMR: affected zones for current alert geo feature: {affected_zones:?}");

        Ok(Self {
            affected_zones,
            status: extract.property("status")?,
            message_type: extract.property("messageType")?,
            sent: extract.property("sent")?,
            effective: extract.property("effective")?,
            onset: extract.property("onset")?,
            expires: extract.property("expires")?,
            ends: extract.property("ends")?,
            category: extract.property("category")?,
            severity: extract.property("severity")?,
            certainty: extract.property("certainty")?,
            urgency: extract.property("urgency")?,
            event: extract.property("event")?,
            headline: extract.property("headline")?,
            description: extract.property("description")?,
            instruction: extract.property("instruction")?,
            response: extract.property("response")?,
        })
    }
}

struct PropertyExtractor<'a> {
    target: &'a str,
    feature: &'a Feature,
}

impl<'a> PropertyExtractor<'a> {
    fn new(target: &'a str, feature: &'a Feature) -> Self {
        Self { target, feature }
    }

    #[instrument(level = "trace", name = "DMR_EXTRACT_PROPERTY", skip(self), ret, err)]
    fn property<T>(&self, property: &str) -> Result<T, WeatherError>
    where
        T: DeserializeOwned + fmt::Debug,
    {
        let p = self.feature.property(property).ok_or_else(|| {
            WeatherError::MissingGeoJsonProperty {
                target: self.target.to_string(),
                property: property.to_string(),
            }
        })?;

        let result = serde_json::from_value(p.clone());
        trace!(
            ?result,
            "DMR: feature property {p_type} {property}={p:?}",
            p_type = tynm::type_namem::<T>(1)
        );
        result.map_err(|err| err.into())
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(ascii_case_insensitive)]
pub enum AlertStatus {
    Actual,
    Exercise,
    System,
    Test,
    Draft,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(ascii_case_insensitive)]
pub enum AlertMessageType {
    Actual,
    Alert,
    Update,
    Cancel,
    Test,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(ascii_case_insensitive)]
#[allow(clippy::upper_case_acronyms)]
pub enum AlertCategory {
    Met,
    Geo,
    Safety,
    Security,
    Rescue,
    Fire,
    Health,
    Env,
    Transport,
    Infra,
    CBRNE,
    Other,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
#[serde(rename_all = "PascalCase")]
pub enum AlertSeverity {
    Extreme,
    Severe,
    Moderate,
    Minor,
    Unknown,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
#[serde(rename_all = "PascalCase")]
pub enum AlertCertainty {
    Observed,
    Likely,
    Possible,
    Unlikely,
    Unknown,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
#[serde(rename_all = "PascalCase")]
pub enum AlertUrgency {
    Immediate,
    Expected,
    Future,
    Past,
    Unknown,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
#[serde(rename_all = "PascalCase")]
pub enum AlertResponse {
    Shelter,
    Evacuate,
    Prepare,
    Execute,
    Avoid,
    Monitor,
    Assess,
    AllClear,
    None,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, ToSchema, Serialize, Deserialize)]
// #[schema(example = json!("360.0"))]
#[serde(transparent)]
#[repr(transparent)]
pub struct Direction(f32);

impl Direction {
    pub fn new(direction: f32) -> Self {
        Self(direction)
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Direction> for f32 {
    fn from(d: Direction) -> Self {
        d.0
    }
}

impl approx::AbsDiffEq for Direction {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        f32::abs_diff_eq(&self.0, &other.0, epsilon)
    }
}

impl approx::RelativeEq for Direction {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon,
    ) -> bool {
        f32::relative_eq(&self.0, &other.0, epsilon, max_relative)
    }
}

#[allow(dead_code)]
pub fn average_direction(directions: &[Direction]) -> Option<Direction> {
    if directions.is_empty() {
        return None;
    }
    let n = directions.len() as f32;
    // let sum_x = directions.iter().map(|d| d.0.to_radians().cos()).sum::<f32>();
    // let sum_y = directions.iter().map(|d| d.0.to_radians().sin()).sum::<f32>();
    // let avg_x = sum_x / n;
    // let avg_y = sum_y / n;
    // Some(Direction((avg_y.atan2(avg_x).to_degrees() + 360.0) % 360.0))
    let sum_c = directions.iter().map(|d| d.0.to_radians().cos()).sum::<f32>();
    let sum_s = directions.iter().map(|d| d.0.to_radians().sin()).sum::<f32>();
    let avg_c = sum_c / n;
    let avg_s = sum_s / n;
    let d = avg_s.atan2(avg_c);
    // let d = match (avg_s, avg_c) {
    //     (s, c) if s > 0.0 && c > 0.0 => { s.atan2(c) },
    //     (s, c) if c < 0.0 => { s.atan2(c) + PI},
    //     (s, c) if s < 0.0 && c > 0.0 => { s.atan2(c) + 2.0 * PI },
    //     _ => 0.0,
    // };
    Some(Direction(d.to_degrees()))
    // Some(Direction((avg_y.atan2(avg_x).to_degrees() + 360.0) % 360.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use claims::assert_some;
    use pretty_assertions::assert_eq;
    // - property test for sane averages
    // proptest! {
    //     #[test]
    //     fn test_average_direction(directions in vec(any::<f32>().prop_filter("valid angle", |d| *d>=0.0 && *d<=360.0), 0..10)) {
    //         let directions: Vec<Direction> = directions.into_iter().map(Direction::new).collect();
    //         let result = average_direction(directions.as_slice());
    //         prop_assert!(
    //             result.map(|r| r.into()).map(|avg: f32| avg >= 0.0 && avg <= 360.0).unwrap_or_else(|| directions.is_empty())
    //             // match result {
    //             //     None => directions.is_empty(),
    //             //     Some(average) => average >= 0.0 && average <= 360.0,
    //             // }
    //         );
    //     }
    // }

    #[test]
    fn test_average_direction_single() {
        let directions = [Direction(90.0)];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(90.0), epsilon = 1e-9);
    }

    #[test]
    fn test_average_direction_opposite() {
        let directions = [Direction(90.0), Direction(270.0)];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(180.0), epsilon = 1e-9);
    }

    #[test]
    fn test_average_direction_not_opposite() {
        let directions = [Direction(45.0), Direction(135.0)];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(90.0), epsilon = 1e-9);
    }

    #[test]
    #[ignore]
    fn test_average_direction_three() {
        let directions = [Direction(0.0), Direction(120.0), Direction(240.0)];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(160.0), epsilon = 1e-9);
    }

    #[test]
    #[ignore]
    fn test_average_direction_multiple() {
        let directions = [
            Direction(0.0),
            Direction(45.0),
            Direction(90.0),
            Direction(360.0),
        ];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(45.0), epsilon = 1e-9);
    }

    #[test]
    #[ignore]
    fn test_average_direction_across_0_360() {
        let directions = [
            Direction(0.0),
            Direction(5.0),
            Direction(355.0),
            Direction(360.0),
        ];
        let actual = assert_some!(average_direction(&directions));
        assert_relative_eq!(actual, Direction(0.0), epsilon = 1e-9);
    }

    #[test]
    fn test_average_direction_empty() {
        let directions: &[Direction] = &[];
        assert_eq!(average_direction(directions), None);
    }
}
