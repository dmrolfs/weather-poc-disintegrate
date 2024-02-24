use super::{QualityControl, QuantitativeValue};
use geojson::{Feature, FeatureCollection};
use iso8601_timestamp::Timestamp;
use rust_decimal::prelude::*;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use strum::{IntoEnumIterator, VariantNames};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr, VariantNames};

#[derive(Debug, PartialEq, Eq, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherFrame {
    pub timestamp: Timestamp,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dewpoint: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_direction: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_speed: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_gust: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub barometric_pressure: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sea_level_pressure: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_temperature_last_24_hours: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_temperature_last_24_hours: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precipitation_last_hour: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precipitation_last_3_hours: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precipitation_last_6_hours: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_humidity: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_chill: Option<QuantitativeValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat_index: Option<QuantitativeValue>,
}

impl From<FeatureCollection> for WeatherFrame {
    fn from(geojson: FeatureCollection) -> Self {
        geojson
            .features
            .into_iter()
            .fold(PropertyAggregations::default(), fold_feature)
            .into()
    }
}

#[derive(Debug)]
struct PropertyAggregations {
    timestamp: Timestamp,
    properties: HashMap<QuantitativeProperty, QuantitativeAggregation>,
}

impl Default for PropertyAggregations {
    fn default() -> Self {
        Self {
            timestamp: Timestamp::now_utc(),
            properties: HashMap::with_capacity(QuantitativeProperty::VARIANTS.len()),
        }
    }
}

impl PropertyAggregations {
    pub fn property(&self, q_prop: &QuantitativeProperty) -> Option<QuantitativeValue> {
        self.properties.get(q_prop).cloned().map(|v| v.into())
    }
}

impl From<PropertyAggregations> for WeatherFrame {
    fn from(agg: PropertyAggregations) -> Self {
        Self {
            timestamp: agg.timestamp,
            temperature: agg.property(&QuantitativeProperty::Temperature),
            dewpoint: agg.property(&QuantitativeProperty::Dewpoint),
            wind_direction: agg.property(&QuantitativeProperty::WindDirection),
            wind_speed: agg.property(&QuantitativeProperty::WindSpeed),
            wind_gust: agg.property(&QuantitativeProperty::WindGust),
            barometric_pressure: agg.property(&QuantitativeProperty::BarometricPressure),
            sea_level_pressure: agg.property(&QuantitativeProperty::SeaLevelPressure),
            visibility: agg.property(&QuantitativeProperty::Visibility),
            max_temperature_last_24_hours: agg
                .property(&QuantitativeProperty::MaxTemperatureLast24Hours),
            min_temperature_last_24_hours: agg
                .property(&QuantitativeProperty::MinTemperatureLast24Hours),
            precipitation_last_hour: agg.property(&QuantitativeProperty::PrecipitationLastHour),
            precipitation_last_3_hours: agg
                .property(&QuantitativeProperty::PrecipitationLast3Hours),
            precipitation_last_6_hours: agg
                .property(&QuantitativeProperty::PrecipitationLast6Hours),
            relative_humidity: agg.property(&QuantitativeProperty::RelativeHumidity),
            wind_chill: agg.property(&QuantitativeProperty::WindChill),
            heat_index: agg.property(&QuantitativeProperty::HeatIndex),
            // temperature:None,
            // dewpoint: None,
            // wind_direction: None,
            // wind_speed: None,
            // wind_gust: None,
            // barometric_pressure: None,
            // sea_level_pressure: None,
            // visibility: None,
            // min_temperature_last_24_hours: None,
            // precipitation_last_hour: None,
            // precipitation_last_3_hours: None,
            // precipitation_last_6_hours:None,
            // relative_humidity: None,
            // wind_chill: None,
            // heat_index:None,
        }
    }
}

// #[tracing::instrument(level = "trace", skip(feature))]
fn fold_feature(mut acc: PropertyAggregations, feature: Feature) -> PropertyAggregations {
    if feature.properties.is_none() {
        return acc;
    }

    // debug!(
    //     "QUANTITATIVE_PROPERTIES = {:?}",
    //     QuantitativeProperty::iter()
    //         .map(|qp| {
    //             let s: &'static str = qp.into();
    //             s
    //         })
    //         .collect::<Vec<_>>()
    // );

    // let acc_props: &mut HashMap<QuantitativeProperty, QuantitativeAggregation> = &mut acc.properties;

    for q_prop in QuantitativeProperty::iter() {
        let prop_name: &'static str = q_prop.into();
        // debug!(
        //     "quantitative_properties: {prop_name} = {:?}",
        //     feature.property(prop_name)
        // );
        if let Some(property) = feature.property(prop_name) {
            match serde_json::from_value::<PropertyDetail>(property.clone()) {
                Ok(detail) => {
                    // debug!("quantitative_properties: property detail = {detail:?}");
                    acc.properties
                        .entry(q_prop)
                        .and_modify(|prop_agg| prop_agg.add_detail(detail.clone()))
                        .or_insert(QuantitativeAggregation::new(detail));
                },
                Err(err) => {
                    tracing::error!(error=?err, "failed to parse property detail: {property:?}");
                },
            }
        }
    }

    acc
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
    EnumIter,
    EnumString,
    VariantNames,
    ToSchema,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "camelCase", ascii_case_insensitive)]
pub enum QuantitativeProperty {
    Temperature,
    Dewpoint,
    WindDirection,
    WindSpeed,
    WindGust,
    BarometricPressure,
    SeaLevelPressure,
    Visibility,
    MaxTemperatureLast24Hours,
    MinTemperatureLast24Hours,
    PrecipitationLastHour,
    PrecipitationLast3Hours,
    PrecipitationLast6Hours,
    RelativeHumidity,
    WindChill,
    HeatIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDetail {
    #[serde(default)]
    value: Option<Decimal>,

    unit_code: String,

    #[serde(default)]
    quality_control: Option<QualityControl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QuantitativeAggregation {
    count: usize,
    value_sum: Decimal,
    max_value: Decimal,
    min_value: Decimal,
    pub unit_code: Cow<'static, str>,
    pub quality_control: QualityControl,
}

impl QuantitativeAggregation {
    pub fn new(detail: PropertyDetail) -> Self {
        Self {
            count: 1,
            value_sum: detail.value.unwrap_or_default(),
            max_value: detail.value.unwrap_or_default(),
            min_value: detail.value.unwrap_or_default(),
            unit_code: detail.unit_code.into(),
            quality_control: detail.quality_control.unwrap_or(QualityControl::X),
        }
    }

    pub fn average_value(&self) -> Decimal {
        self.value_sum / try_usize_to_decimal(self.count)
    }

    pub fn add_detail(&mut self, detail: PropertyDetail) {
        // combination strategy:
        // lessor rhs quality control => ignore value
        // same rhs quality control => combine avg and min/max
        // higher rhs quality control => reset aggregation with rhs

        if let Some((value, quality)) = detail.value.zip(detail.quality_control) {
            match self.quality_control.cmp(&quality) {
                Ordering::Less => (),

                Ordering::Greater => {
                    self.count = 1;
                    self.value_sum = value;
                    self.max_value = value;
                    self.min_value = value;
                    self.quality_control = quality;
                },

                Ordering::Equal => {
                    self.count += 1;
                    self.value_sum += value;
                    self.max_value = value.max(self.max_value);
                    self.min_value = value.min(self.min_value);
                },
            }
        }
    }
}

#[inline]
fn try_usize_to_decimal(value: usize) -> Decimal {
    Decimal::from(value)
}

// impl std::ops::Add<PropertyDetail> for QuantitativeAggregation {
//     type Output = Self;
//
//     fn add(self, rhs: PropertyDetail) -> Self::Output {
//         // combination strategy:
//         // lessor rhs quality control => ignore value
//         // same rhs quality control => combine avg and min/max
//         // higher rhs quality control => reset aggregation with rhs
//
//         match self.quality_control.cmp(&rhs.quality_control) {
//             Ordering::Less => self,
//             Ordering::Greater => Self {
//                 count: 1,
//                 value_sum: rhs.value,
//                 max_value: rhs.value,
//                 min_value: rhs.value,
//                 quality_control: rhs.quality_control,
//                 ..self
//             },
//             Ordering::Equal => Self {
//                 count: self.count + 1,
//                 value_sum: self.value_sum + rhs.value,
//                 max_value: self.max_value.max(rhs.value),
//                 min_value: self.min_value.min(rhs.value),
//                 ..self
//             },
//         }
//     }
// }

impl From<QuantitativeAggregation> for QuantitativeValue {
    fn from(agg: QuantitativeAggregation) -> Self {
        Self {
            value: agg.average_value(),
            max_value: agg.max_value,
            min_value: agg.min_value,
            unit_code: agg.unit_code,
            quality_control: agg.quality_control,
        }
    }
}
