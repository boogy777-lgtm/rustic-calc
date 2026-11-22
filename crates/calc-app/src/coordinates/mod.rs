//! Offline WGS84 → local Transverse Mercator coordinates. No platform or I/O.
//!
//! This is a deliberately bounded PROJ-string subset, not a PROJ replacement.
//! See docs/COORDINATES.md for conventions, provenance and accuracy limits.

pub mod catalogue;
mod projection;

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    EmptyDefinition,
    Missing(&'static str),
    Invalid(String),
    Unsupported(String),
    Duplicate(String),
    DatumRequired,
    Coordinate(CoordinateField),
    OutsideProjection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordinateField {
    Latitude,
    Longitude,
    Height,
}

#[derive(Debug, Clone, Copy)]
pub struct Wgs84 {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub height_m: f64,
}

impl Wgs84 {
    /// Decimal degrees and WGS84 ellipsoidal metres. A comma decimal separator
    /// is accepted in coordinate fields, never in PROJ numeric parameters.
    pub fn parse(latitude: &str, longitude: &str, height: &str) -> Result<Self, Error> {
        fn number(text: &str, field: CoordinateField) -> Result<f64, Error> {
            text.trim()
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or(Error::Coordinate(field))
        }
        let point = Self {
            latitude_deg: number(latitude, CoordinateField::Latitude)?,
            longitude_deg: number(longitude, CoordinateField::Longitude)?,
            height_m: number(height, CoordinateField::Height)?,
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(self) -> Result<(), Error> {
        for (value, valid, field) in [
            (
                self.latitude_deg,
                self.latitude_deg.abs() < 90.0,
                CoordinateField::Latitude,
            ),
            (
                self.longitude_deg,
                self.longitude_deg.abs() <= 180.0,
                CoordinateField::Longitude,
            ),
            (
                self.height_m,
                (-10_000.0..=100_000.0).contains(&self.height_m),
                CoordinateField::Height,
            ),
        ] {
            if !value.is_finite() || !valid {
                return Err(Error::Coordinate(field));
            }
        }
        Ok(())
    }
}

/// Russian surveying order: X = northing, Y = easting, both in metres.
#[derive(Debug, Clone, Copy)]
pub struct LocalPoint {
    pub northing_m: f64,
    pub easting_m: f64,
}

#[derive(Debug, Clone, Copy)]
struct Ellipsoid {
    a: f64,
    rf: f64,
}

const WGS84: Ellipsoid = Ellipsoid {
    a: 6_378_137.0,
    rf: 298.257223563,
};

impl Ellipsoid {
    fn eccentricity_squared(self) -> f64 {
        let f = 1.0 / self.rf;
        f * (2.0 - f)
    }

    fn geocentric(self, latitude: f64, longitude: f64, height: f64) -> [f64; 3] {
        let (sin_lat, cos_lat) = latitude.sin_cos();
        let (sin_lon, cos_lon) = longitude.sin_cos();
        let e2 = self.eccentricity_squared();
        let n = self.a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
        [
            (n + height) * cos_lat * cos_lon,
            (n + height) * cos_lat * sin_lon,
            (n * (1.0 - e2) + height) * sin_lat,
        ]
    }

    fn geodetic(self, xyz: [f64; 3]) -> Result<(f64, f64), Error> {
        let [x, y, z] = xyz;
        let p = x.hypot(y);
        let e2 = self.eccentricity_squared();
        let mut lat = z.atan2(p * (1.0 - e2));
        for _ in 0..20 {
            let n = self.a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
            let next = (z + e2 * n * lat.sin()).atan2(p);
            if (next - lat).abs() < 1e-14 {
                return Ok((next, y.atan2(x)));
            }
            lat = next;
        }
        Err(Error::OutsideProjection)
    }
}

#[derive(Debug, Clone)]
pub struct Crs {
    ellipsoid: Ellipsoid,
    latitude_origin: f64,
    longitude_origin: f64,
    scale: f64,
    false_easting: f64,
    false_northing: f64,
    /// Local → WGS84 position-vector convention, rotations in arcseconds,
    /// scale in ppm. Conversion applies its legacy small-angle inverse.
    towgs84: [f64; 7],
}

impl Crs {
    pub fn parse(definition: &str) -> Result<Self, Error> {
        let definition = definition.trim();
        if definition.is_empty() {
            return Err(Error::EmptyDefinition);
        }
        if definition.len() > 16_384 || !definition.starts_with('+') {
            return Err(Error::Invalid("PROJ".into()));
        }
        // Split on '+' rather than whitespace: towgs84 may contain spaces
        // after commas, and pasted definitions may span multiple lines.
        let mut values = BTreeMap::new();
        for token in definition[1..].split('+') {
            let (key, value) = token.trim().split_once('=').unwrap_or((token.trim(), ""));
            if !matches!(
                key,
                "proj"
                    | "lat_0"
                    | "lon_0"
                    | "k"
                    | "k_0"
                    | "x_0"
                    | "y_0"
                    | "ellps"
                    | "a"
                    | "rf"
                    | "towgs84"
                    | "datum"
                    | "units"
                    | "no_defs"
                    | "type"
            ) {
                return Err(Error::Unsupported(key.chars().take(60).collect()));
            }
            if values.insert(key, value.trim()).is_some() {
                return Err(Error::Duplicate(key.into()));
            }
        }
        let required = |key: &'static str| values.get(key).copied().ok_or(Error::Missing(key));
        let numeric = |key: &'static str, text: &str| {
            text.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| Error::Invalid(key.into()))
        };
        if required("proj")? != "tmerc" {
            return Err(Error::Unsupported("proj (tmerc)".into()));
        }
        if required("units")? != "m" {
            return Err(Error::Unsupported("units (m)".into()));
        }
        for (key, expected) in [("no_defs", ""), ("type", "crs")] {
            if values.get(key).is_some_and(|value| *value != expected) {
                return Err(Error::Invalid(key.into()));
            }
        }
        let ellipsoid = if let Some(name) = values.get("ellps") {
            if values.contains_key("a") || values.contains_key("rf") {
                return Err(Error::Duplicate("ellps / a,rf".into()));
            }
            match *name {
                "krass" => Ellipsoid {
                    a: 6_378_245.0,
                    rf: 298.3,
                },
                "bessel" => Ellipsoid {
                    a: 6_377_397.155,
                    rf: 299.1528128,
                },
                "WGS84" => WGS84,
                "GRS80" => Ellipsoid {
                    a: 6_378_137.0,
                    rf: 298.257222101,
                },
                _ => return Err(Error::Unsupported("ellps".into())),
            }
        } else if values.contains_key("a") || values.contains_key("rf") {
            Ellipsoid {
                a: numeric("a", required("a")?)?,
                rf: numeric("rf", required("rf")?)?,
            }
        } else if values.get("datum") == Some(&"WGS84") {
            WGS84
        } else {
            return Err(Error::Missing("ellps / a,rf"));
        };
        if !(6_000_000.0..=7_000_000.0).contains(&ellipsoid.a)
            || !(200.0..=400.0).contains(&ellipsoid.rf)
        {
            return Err(Error::Invalid("ellps / a,rf".into()));
        }
        let mut towgs84 = [0.0; 7];
        if let Some(datum) = values.get("datum") {
            if *datum != "WGS84" {
                return Err(Error::Unsupported("datum".into()));
            }
            if values.contains_key("towgs84") {
                return Err(Error::Duplicate("datum / towgs84".into()));
            }
            if ellipsoid.a != WGS84.a || ellipsoid.rf != WGS84.rf {
                return Err(Error::Invalid("datum / ellps".into()));
            }
        } else if let Some(shift) = values.get("towgs84") {
            let params: Vec<_> = shift.split(',').collect();
            if params.len() != 3 && params.len() != 7 {
                return Err(Error::Invalid("towgs84 (3 / 7)".into()));
            }
            for (i, text) in params.iter().enumerate() {
                towgs84[i] = numeric("towgs84", text.trim())?;
            }
            if towgs84[..3].iter().any(|v| v.abs() > 1_000_000.0)
                || towgs84[3..6].iter().any(|v| v.abs() > 60.0)
                || towgs84[6].abs() > 10_000.0
            {
                return Err(Error::Invalid("towgs84".into()));
            }
        } else {
            // An ellipsoid alone is not a datum. Never assume zero shifts.
            return Err(Error::DatumRequired);
        }
        if values.contains_key("k") && values.contains_key("k_0") {
            return Err(Error::Duplicate("k / k_0".into()));
        }
        let scale_key = if values.contains_key("k") { "k" } else { "k_0" };
        let crs = Self {
            ellipsoid,
            latitude_origin: numeric("lat_0", required("lat_0")?)?.to_radians(),
            longitude_origin: numeric("lon_0", required("lon_0")?)?.to_radians(),
            scale: numeric(scale_key, required(scale_key)?)?,
            false_easting: numeric("x_0", required("x_0")?)?,
            false_northing: numeric("y_0", required("y_0")?)?,
            towgs84,
        };
        if crs.latitude_origin.abs() >= std::f64::consts::FRAC_PI_2 {
            return Err(Error::Invalid("lat_0".into()));
        }
        if crs.longitude_origin.abs() > std::f64::consts::TAU {
            return Err(Error::Invalid("lon_0".into()));
        }
        if !(0.0 < crs.scale && crs.scale <= 2.0) {
            return Err(Error::Invalid(scale_key.into()));
        }
        if crs.false_easting.abs() > 1e9 || crs.false_northing.abs() > 1e9 {
            return Err(Error::Invalid("x_0 / y_0".into()));
        }
        Ok(crs)
    }

    pub fn convert(&self, point: Wgs84) -> Result<LocalPoint, Error> {
        point.validate()?;
        let mut latitude = point.latitude_deg.to_radians();
        let mut longitude = point.longitude_deg.to_radians();
        if self.ellipsoid.a != WGS84.a
            || self.ellipsoid.rf != WGS84.rf
            || self.towgs84.iter().any(|v| *v != 0.0)
        {
            let xyz = WGS84.geocentric(latitude, longitude, point.height_m);
            let [dx, dy, dz, rx, ry, rz, ppm] = self.towgs84;
            let arcsec = std::f64::consts::PI / (180.0 * 3600.0);
            let (rx, ry, rz) = (rx * arcsec, ry * arcsec, rz * arcsec);
            let m = 1.0 + ppm * 1e-6;
            let (x, y, z) = ((xyz[0] - dx) / m, (xyz[1] - dy) / m, (xyz[2] - dz) / m);
            let local = [
                x + rz * y - ry * z,
                -rz * x + y + rx * z,
                ry * x - rx * y + z,
            ];
            (latitude, longitude) = self.ellipsoid.geodetic(local)?;
        }
        projection::forward(self, latitude, longitude)
    }
}

#[cfg(test)]
mod tests;
