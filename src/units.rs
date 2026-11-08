//! Unit conversion, date calculation and worksheet formulas.
//!
//! Self-contained numerical tables and formulas. No third-party executable,
//! MUI resource or data file is loaded or distributed with Rustic Calc.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

fn leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
        {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn add_months(self, months: i32) -> Option<Self> {
        let total = (self.year * 12 + self.month as i32 - 1).checked_add(months)?;
        let year = total.div_euclid(12);
        let month = (total.rem_euclid(12) + 1) as u32;
        if !(1..=9999).contains(&year) {
            return None;
        }
        let day = self.day.min(days_in_month(year, month));
        Some(Self { year, month, day })
    }

    pub fn add_days(self, days: i64) -> Option<Self> {
        let serial = days_from_civil(self.year, self.month, self.day).checked_add(days)?;
        if !(days_from_civil(1, 1, 1)..=days_from_civil(9999, 12, 31)).contains(&serial) {
            return None;
        }
        let (year, month, day) = civil_from_days(serial);
        if !(1..=9999).contains(&year) {
            return None;
        }
        Some(Self { year, month, day })
    }
}

/// Days since 1970-01-01 (Howard Hinnant's civil calendar algorithm).
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i32;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as i64
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    ((y + if m <= 2 { 1 } else { 0 }) as i32, m as u32, d as u32)
}

#[derive(Clone, Copy)]
pub struct Unit {
    pub name: &'static str,
    pub factor: f64,
    pub offset: f64,
}

impl Unit {
    const fn linear(name: &'static str, factor: f64) -> Self {
        Self {
            name,
            factor,
            offset: 0.0,
        }
    }

    const fn affine(name: &'static str, factor: f64, offset: f64) -> Self {
        Self {
            name,
            factor,
            offset,
        }
    }

    fn to_base(self, value: f64) -> f64 {
        value * self.factor + self.offset
    }

    fn from_base(self, value: f64) -> f64 {
        (value - self.offset) / self.factor
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryId {
    Length,
    Mass,
    Volume,
    Temperature,
    Angle,
    Pressure,
    Flow,
    MassFlow,
    DcsSignals,
    Force,
    Torque,
    Density,
    RotationalSpeed,
    Velocity,
    Area,
    Energy,
    Power,
    Time,
}

pub struct Category {
    pub id: CategoryId,
    pub name: &'static str,
    pub units: Vec<Unit>,
}

pub fn categories() -> Vec<Category> {
    vec![
        Category {
            id: CategoryId::Length,
            name: "Length",
            units: vec![
                Unit::linear("Angstrom", 0.00000001),
                Unit::linear("Nanometer", 0.0000001),
                Unit::linear("Microns", 0.0001),
                Unit::linear("Millimeters", 0.1),
                Unit::linear("Centimeters", 1.0),
                Unit::linear("Meter", 100.0),
                Unit::linear("Kilometers", 100000.0),
                Unit::linear("Inch", 2.54),
                // template: 165 / 10 * 254 / 100 * 12 = 502.92 cm
                Unit::linear("Rods", 502.92),
                Unit::linear("Feet", 30.48),
                Unit::linear("Yard", 91.44),
                Unit::linear("Fathom", 182.88),
                Unit::linear("Mile", 160934.4),
                Unit::linear("Nautical Miles", 185200.0),
                // template: 12 * 254 / 100 * 13837 / 1000000
                Unit::linear("PICA", 0.421_751_76),
                Unit::linear("Hand", 10.16),
                Unit::linear("Link", 20.1168),
                Unit::linear("Chain", 201.168),
                Unit::linear("Span", 22.86),
            ],
        },
        Category {
            id: CategoryId::Mass,
            name: "Weight/Mass",
            units: vec![
                Unit::linear("Milligram", 0.001),
                Unit::linear("Centigram", 0.01),
                Unit::linear("Decigram", 0.1),
                Unit::linear("Dekagram", 10.0),
                Unit::linear("Hectogram", 100.0),
                Unit::linear("Gram", 1.0),
                Unit::linear("Kilogram", 1000.0),
                Unit::linear("Tonne", 1_000_000.0),
                Unit::linear("Ounce", 28.349523125),
                Unit::linear("Pound", 453.59237),
                Unit::linear("Stone", 6350.29318),
                Unit::linear("Short ton", 907184.74),
                Unit::linear("Long ton", 1016046.9088),
                Unit::linear("Carat", 0.2),
            ],
        },
        Category {
            id: CategoryId::Volume,
            name: "Volume",
            units: vec![
                Unit::linear("Cubic centimeter", 0.001),
                Unit::linear("Liter", 1.0),
                Unit::linear("Cubic meter", 1000.0),
                Unit::linear("Cubic inch", 0.016387064),
                Unit::linear("Fluid ounce (UK)", 0.0284130625),
                Unit::linear("Fluid ounce (US)", 0.0295735295625),
                Unit::linear("Pint (US)", 0.473176473),
                Unit::linear("Pint (UK)", 0.56826125),
                Unit::linear("Quart (UK)", 1.1365225),
                Unit::linear("Quart (US)", 0.946352946),
                Unit::linear("Gallon (UK)", 4.54609),
                Unit::linear("Gallon (US)", 3.785411784),
                Unit::linear("Cubic feet", 28.316846592),
                Unit::linear("Cubic yard", 764.554857984),
                Unit::linear("Milliliter", 0.001),
                Unit::linear("Barrel (oil)", 158.987294928),
            ],
        },
        Category {
            id: CategoryId::Temperature,
            name: "Temperature",
            units: vec![
                Unit {
                    name: "Degrees Celsius",
                    factor: 1.0,
                    offset: 273.15,
                },
                Unit {
                    name: "Degrees Fahrenheit",
                    factor: 5.0 / 9.0,
                    offset: 255.372_222_222_222_2,
                },
                Unit {
                    name: "Kelvin",
                    factor: 1.0,
                    offset: 0.0,
                },
                Unit::linear("Degrees Rankine", 5.0 / 9.0),
            ],
        },
        Category {
            id: CategoryId::Angle,
            name: "Angle",
            units: vec![
                Unit::linear("Degree", 1.0),
                Unit::linear("Gradian", 0.9),
                Unit::linear("Radian", 57.29577951308232),
            ],
        },
        Category {
            id: CategoryId::Pressure,
            name: "Pressure",
            units: vec![
                Unit::linear("Pascal", 1.0),
                Unit::linear("Kilo Pascal", 1000.0),
                Unit::linear("Millimeter of mercury", 133.3),
                Unit::linear("Bar", 100000.0),
                Unit::linear("Atmosphere", 101325.0),
                Unit::linear("Pound per square inch (PSI)", 6894.757),
                Unit::linear("Mega Pascal", 1_000_000.0),
                Unit::linear("Kilogram-force per square centimeter", 98_066.5),
                Unit::linear("Millimeter of water", 9.80665),
                Unit::linear("Inch of water", 249.08891),
                Unit::linear("Inch of mercury", 3386.389),
                Unit::linear("Pound per square foot", 47.88025898033584),
            ],
        },
        Category {
            id: CategoryId::Flow,
            name: "Flow",
            units: vec![
                Unit::linear("Cubic meter per second", 1.0),
                Unit::linear("Cubic meter per minute", 1.0 / 60.0),
                Unit::linear("Cubic meter per hour", 1.0 / 3600.0),
                Unit::linear("Liter per second", 0.001),
                Unit::linear("Liter per minute", 1.0 / 60_000.0),
                Unit::linear("Liter per hour", 1.0 / 3_600_000.0),
                Unit::linear("US gallon per minute (GPM)", 0.0000630901964),
                Unit::linear("US gallon per hour", 0.0000010515032733333334),
                Unit::linear("Cubic foot per minute (CFM)", 0.0004719474432),
                Unit::linear("Cubic foot per hour", 0.00000786579072),
                Unit::linear("Barrel per day (bbl/d)", 0.0000018401307283333333),
                Unit::linear("Imperial gallon per minute", 0.00007576816666666667),
            ],
        },
        Category {
            id: CategoryId::MassFlow,
            name: "Mass Flow",
            units: vec![
                Unit::linear("Kilogram per second", 1.0),
                Unit::linear("Kilogram per minute", 1.0 / 60.0),
                Unit::linear("Kilogram per hour", 1.0 / 3600.0),
                Unit::linear("Tonne per hour", 1000.0 / 3600.0),
                Unit::linear("Tonne per day", 1000.0 / 86400.0),
                Unit::linear("Pound per second", 0.45359237),
                Unit::linear("Pound per minute", 0.45359237 / 60.0),
                Unit::linear("Pound per hour", 0.45359237 / 3600.0),
            ],
        },
        Category {
            id: CategoryId::DcsSignals,
            name: "DCS Signals",
            units: vec![
                Unit::linear("Percent of span (%)", 1.0),
                Unit::affine("Current (4-20 mA)", 6.25, -25.0),
                Unit::affine("Current (0-20 mA)", 5.0, 0.0),
                Unit::affine("Voltage (1-5 V)", 25.0, -25.0),
                Unit::affine("Voltage (2-10 V)", 12.5, -25.0),
                Unit::affine("Voltage (0-10 V)", 10.0, 0.0),
                Unit::affine("Pneumatic (3-15 psi)", 25.0 / 3.0, -25.0),
                Unit::linear("Siemens raw (0-27648)", 100.0 / 27_648.0),
                Unit::linear("Raw (0-32767)", 100.0 / 32_767.0),
            ],
        },
        Category {
            id: CategoryId::Force,
            name: "Force",
            units: vec![
                Unit::linear("Newton", 1.0),
                Unit::linear("Kilonewton", 1000.0),
                Unit::linear("Dekanewton", 10.0),
                Unit::linear("Kilogram-force", 9.80665),
                Unit::linear("Pound-force", 4.4482216152605),
                Unit::linear("Kip", 4448.2216152605),
            ],
        },
        Category {
            id: CategoryId::Torque,
            name: "Torque",
            units: vec![
                Unit::linear("Newton meter", 1.0),
                Unit::linear("Kilonewton meter", 1000.0),
                Unit::linear("Kilogram-force meter", 9.80665),
                Unit::linear("Kilogram-force centimeter", 0.0980665),
                Unit::linear("Pound-force foot", 1.3558179483314),
                Unit::linear("Pound-force inch", 0.1129848290276167),
            ],
        },
        Category {
            id: CategoryId::Density,
            name: "Density",
            units: vec![
                Unit::linear("Kilogram per cubic meter", 1.0),
                Unit::linear("Gram per cubic centimeter", 1000.0),
                Unit::linear("Kilogram per liter", 1000.0),
                Unit::linear("Pound per cubic foot", 16.01846337396014),
                Unit::linear("Pound per US gallon", 119.82642731689663),
                Unit::linear("Specific gravity (water = 1)", 1000.0),
            ],
        },
        Category {
            id: CategoryId::RotationalSpeed,
            name: "Rotational Speed",
            units: vec![
                Unit::linear("Revolution per minute (rpm)", 1.0),
                Unit::linear("Revolution per second (rps)", 60.0),
                Unit::linear("Radian per second", 9.549296585513721),
                Unit::linear("Hertz", 60.0),
            ],
        },
        Category {
            id: CategoryId::Velocity,
            name: "Velocity",
            units: vec![
                Unit::linear("Centimeter per second", 0.01),
                Unit::linear("Miles per hour", 0.44704),
                Unit::linear("Feet per second", 0.3048),
                Unit::linear("Meter per second", 1.0),
                Unit::linear("Kilometer per hour", 0.2777777777777778),
                Unit::linear("Knots", 0.5144444444444445),
                Unit::linear("Mach (at std. atm)", 340.2933),
                Unit::linear("Millimeter per second", 0.001),
                Unit::linear("Meter per minute", 1.0 / 60.0),
                Unit::linear("Feet per minute", 0.00508),
            ],
        },
        Category {
            id: CategoryId::Area,
            name: "Area",
            units: vec![
                Unit::linear("Square millimeter", 0.01),
                Unit::linear("Square centimeter", 1.0),
                Unit::linear("Square inch", 6.4516),
                Unit::linear("Square feet", 929.0304),
                Unit::linear("Hectares", 100_000_000.0),
                Unit::linear("Square kilometer", 10_000_000_000.0),
                Unit::linear("Square Yard", 8361.2736),
                Unit::linear("Acres", 40_468_564.224),
                Unit::linear("Square mile", 25_899_881_103.36),
                Unit::linear("Square meters", 10000.0),
            ],
        },
        Category {
            id: CategoryId::Energy,
            name: "Energy",
            units: vec![
                Unit::linear("Joule", 1.0),
                Unit::linear("Kilojoule", 1000.0),
                Unit::linear("Calorie", 4.1868),
                Unit::linear("Kilocalorie", 4186.8),
                Unit::linear("British Thermal Unit", 1055.056),
                Unit::linear("Foot-Pound", 1.3558179483314),
                Unit::linear("Electron-Volts", 1.60217653e-19),
                Unit::linear("Watt-hour", 3600.0),
                Unit::linear("Kilowatt-hour", 3_600_000.0),
                Unit::linear("Megawatt-hour", 3_600_000_000.0),
            ],
        },
        Category {
            id: CategoryId::Power,
            name: "Power",
            units: vec![
                Unit::linear("Watt", 1.0),
                Unit::linear("Horsepower", 745.6998715822702),
                Unit::linear("Kilowatt", 1000.0),
                Unit::linear("BTU/minute", 17.584266666666667),
                Unit::linear("Foot-Pound/minute", 0.0225969658055233),
                Unit::linear("Megawatt", 1_000_000.0),
                Unit::linear("BTU/hour", 0.2930711111111111),
                Unit::linear("Kilocalorie/hour", 1.163),
                Unit::linear("Ton of refrigeration", 3516.8533333333335),
            ],
        },
        Category {
            id: CategoryId::Time,
            name: "Time",
            units: vec![
                Unit::linear("Microsecond", 0.000001),
                Unit::linear("Millisecond", 0.001),
                Unit::linear("Second", 1.0),
                Unit::linear("Minute", 60.0),
                Unit::linear("Hour", 3600.0),
                Unit::linear("Day", 86400.0),
                Unit::linear("Week", 604800.0),
            ],
        },
    ]
}

pub fn convert(value: f64, from: &Unit, to: &Unit) -> f64 {
    to.from_base(from.to_base(value))
}

/// Resistor selection for a DCS/process current loop.
#[derive(Clone, Copy, Debug)]
pub struct LoopResistor {
    pub resistance: f64,
    pub power: f64,
    pub standard_resistance: f64,
    pub actual_current_ma: f64,
    pub recommended_wattage: f64,
}

const E24: [f64; 24] = [
    1.0, 1.1, 1.2, 1.3, 1.5, 1.6, 1.8, 2.0, 2.2, 2.4, 2.7, 3.0, 3.3, 3.6, 3.9, 4.3, 4.7, 5.1, 5.6,
    6.2, 6.8, 7.5, 8.2, 9.1,
];

const POWER_RATINGS: [f64; 10] = [0.125, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0];

/// Calculates `R = U / I`, dissipation and the nearest E24 resistor.
/// `voltage` is in volts, `current_ma` is in milliamps.
pub fn loop_resistor(voltage: f64, current_ma: f64) -> Option<LoopResistor> {
    if !voltage.is_finite() || !current_ma.is_finite() || voltage <= 0.0 || current_ma <= 0.0 {
        return None;
    }
    let resistance = voltage * 1000.0 / current_ma;
    let power = voltage * current_ma / 1000.0;
    if !resistance.is_finite() || !power.is_finite() || resistance <= 0.0 {
        return None;
    }
    let standard_resistance = nearest_e24(resistance);
    let actual_current_ma = voltage * 1000.0 / standard_resistance;
    if !standard_resistance.is_finite()
        || !actual_current_ma.is_finite()
        || standard_resistance <= 0.0
    {
        return None;
    }
    Some(LoopResistor {
        resistance,
        power,
        standard_resistance,
        actual_current_ma,
        recommended_wattage: recommended_wattage(power),
    })
}

fn nearest_e24(value: f64) -> f64 {
    let decade = 10f64.powf(value.log10().floor());
    let normalized = value / decade;
    let mut best = E24[0];
    let mut best_diff = (best - normalized).abs();
    for candidate in E24.iter().copied().chain(std::iter::once(10.0)) {
        let diff = (candidate - normalized).abs();
        if diff < best_diff {
            best = candidate;
            best_diff = diff;
        }
    }
    best * decade
}

fn recommended_wattage(power: f64) -> f64 {
    let target = power * 1.5;
    for rating in POWER_RATINGS {
        if rating >= target {
            return rating;
        }
    }
    POWER_RATINGS[POWER_RATINGS.len() - 1]
}

/// Formats a resistance using Ω, kΩ or MΩ as appropriate.
pub fn format_resistance(ohms: f64, grouping: bool) -> String {
    if !ohms.is_finite() || ohms <= 0.0 {
        return "—".to_string();
    }
    if ohms >= 1_000_000_000.0 {
        format!("{} GΩ", crate::engine::format_number(ohms / 1e9, grouping))
    } else if ohms >= 1_000_000.0 {
        format!("{} MΩ", crate::engine::format_number(ohms / 1e6, grouping))
    } else if ohms >= 1000.0 {
        format!(
            "{} kΩ",
            crate::engine::format_number(ohms / 1000.0, grouping)
        )
    } else {
        format!("{} Ω", crate::engine::format_number(ohms, grouping))
    }
}

// ---------------------------------------------------------------------------
// Worksheets
// ---------------------------------------------------------------------------

pub struct WorksheetDef {
    pub args: &'static [&'static str],
    pub default_target: usize,
    pub non_computable: &'static [usize],
}

pub fn worksheet_def(kind: crate::engine::WorksheetKind) -> WorksheetDef {
    use crate::engine::WorksheetKind::*;
    match kind {
        Mortgage => WorksheetDef {
            args: &[
                "Purchase price",
                "Down payment",
                "Term (years)",
                "Interest rate (%)",
                "Monthly payment",
            ],
            default_target: 4,
            non_computable: &[3],
        },
        VehicleLease => WorksheetDef {
            args: &[
                "Lease value",
                "Lease period",
                "Payments per year",
                "Residual value",
                "Interest rate (%)",
                "Periodic payment",
            ],
            default_target: 5,
            non_computable: &[2, 4],
        },
        FuelMpg => WorksheetDef {
            args: &[
                "Distance (miles)",
                "Fuel used (gallons)",
                "Fuel economy (mpg)",
            ],
            default_target: 2,
            non_computable: &[],
        },
        FuelL100 => WorksheetDef {
            args: &[
                "Distance (kilometers)",
                "Fuel used (liters)",
                "Fuel economy (L/100 km)",
            ],
            default_target: 2,
            non_computable: &[],
        },
    }
}

/// values contains the input value for every argument, or NaN for the target.
pub fn solve_worksheet(
    kind: crate::engine::WorksheetKind,
    target: usize,
    values: &[f64],
) -> Result<f64, String> {
    use crate::engine::WorksheetKind::*;
    let get = |i: usize| -> Result<f64, String> {
        let v = *values.get(i).unwrap_or(&f64::NAN);
        if !v.is_finite() {
            return Err("This value cannot be blank.".to_string());
        }
        Ok(v)
    };
    let positive = |i: usize| -> Result<f64, String> {
        let v = get(i)?;
        if v <= 0.0 {
            return Err("This value is not valid.".to_string());
        }
        Ok(v)
    };

    let result = match kind {
        Mortgage => {
            let purchase = if target == 0 { 0.0 } else { positive(0)? };
            let down = if target == 1 { 0.0 } else { get(1)? };
            if down < 0.0 || (target != 0 && target != 1 && down >= purchase) {
                return Err("This value is not valid.".to_string());
            }
            let term = if target == 2 { 1.0 } else { positive(2)? };
            let interest = get(3)?;
            if interest < 0.0 {
                return Err("This value is not valid.".to_string());
            }
            let n = term * 12.0;
            let r = interest / 1200.0;
            let principal = purchase - down;
            let factor = if r.abs() < 1e-15 {
                n
            } else {
                (1.0 - (1.0 / (1.0 + r).powf(n))) / r
            };
            match target {
                0 => {
                    // purchase price
                    let m = positive(4)?;
                    m * factor + down
                }
                1 => {
                    let m = positive(4)?;
                    purchase - m * factor
                }
                2 => {
                    let m = positive(4)?;
                    if r.abs() < 1e-15 {
                        principal / m / 12.0
                    } else {
                        (1.0 / (1.0 - (principal * r / m))).ln() / (1.0 + r).ln() / 12.0
                    }
                }
                3 => return Err("This value is not valid.".to_string()),
                4 => principal / factor,
                _ => return Err("This value is not valid.".to_string()),
            }
        }
        VehicleLease => {
            let lease_value = if target == 0 { 0.0 } else { positive(0)? };
            let period = if target == 1 { 1.0 } else { positive(1)? };
            let payments = positive(2)?;
            let residual = if target == 3 { 0.0 } else { get(3)? };
            let interest = get(4)?;
            if interest < 0.0 || residual < 0.0 {
                return Err("This value is not valid.".to_string());
            }
            let r = interest / payments / 100.0;
            let n = payments * period;
            let factor = if r.abs() < 1e-15 {
                n
            } else {
                (1.0 - (1.0 / (1.0 + r).powf(n))) / r
            };
            let discount = 1.0 / (1.0 + r).powf(n);
            match target {
                0 => {
                    let m = positive(5)?;
                    m * factor + residual * discount
                }
                1 => {
                    let m = positive(5)?;
                    let base = residual - m / r;
                    let num = lease_value - m / r;
                    if r.abs() < 1e-15 {
                        (lease_value - residual) / m / payments
                    } else {
                        (base / num).ln() / (1.0 + r).ln() / payments
                    }
                }
                2 => return Err("This value is not valid.".to_string()),
                3 => {
                    let m = positive(5)?;
                    ((1.0 + r).powf(n)) * (lease_value - m * factor)
                }
                4 => return Err("This value is not valid.".to_string()),
                5 => (lease_value - residual * discount) / factor,
                _ => return Err("This value is not valid.".to_string()),
            }
        }
        FuelMpg => {
            let distance = if target == 0 { 0.0 } else { positive(0)? };
            let fuel = if target == 1 { 0.0 } else { positive(1)? };
            let mpg = if target == 2 { 0.0 } else { positive(2)? };
            match target {
                0 => fuel * mpg,
                1 => distance / mpg,
                2 => distance / fuel,
                _ => return Err("This value is not valid.".to_string()),
            }
        }
        FuelL100 => {
            let distance = if target == 0 { 0.0 } else { positive(0)? };
            let fuel = if target == 1 { 0.0 } else { positive(1)? };
            let per100 = if target == 2 { 0.0 } else { positive(2)? };
            match target {
                0 => fuel * 100.0 / per100,
                1 => distance * per100 / 100.0,
                2 => fuel * 100.0 / distance,
                _ => return Err("This value is not valid.".to_string()),
            }
        }
    };
    let permits_zero = (kind == Mortgage && target == 1) || (kind == VehicleLease && target == 3);
    if !result.is_finite() || result < 0.0 || (result == 0.0 && !permits_zero) {
        return Err("Unable to calculate".to_string());
    }
    if kind == Mortgage
        && ((target == 0 && result <= values[1]) || (target == 1 && result >= values[0]))
    {
        return Err("Unable to calculate".to_string());
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Date calculation
// ---------------------------------------------------------------------------

pub fn parse_date(text: &str) -> Option<Date> {
    let text = text.trim();
    let parts: Vec<&str> = if text.contains('.') {
        text.split('.').collect()
    } else if text.contains('/') {
        text.split('/').collect()
    } else if text.contains('-') {
        text.split('-').collect()
    } else {
        return None;
    };
    if parts.len() != 3 {
        return None;
    }
    let (year, month, day) = if parts[0].len() == 4 {
        (
            parts[0].parse::<i32>().ok()?,
            parts[1].parse::<u32>().ok()?,
            parts[2].parse::<u32>().ok()?,
        )
    } else {
        (
            parts[2].parse::<i32>().ok()?,
            parts[1].parse::<u32>().ok()?,
            parts[0].parse::<u32>().ok()?,
        )
    };
    Date::new(year, month, day)
}

pub fn format_date(date: Date) -> String {
    format!("{:02}.{:02}.{:04}", date.day, date.month, date.year)
}

pub fn today() -> Date {
    let (year, month, day) = crate::platform::local_date();
    Date::new(year, month, day).unwrap_or(Date {
        year: 1970,
        month: 1,
        day: 1,
    })
}

/// Returns (years, months, weeks, days, total_days) for the absolute difference.
pub fn date_difference(from: Date, to: Date) -> (i32, i32, i32, i32, i64) {
    let (start, end) = if from <= to { (from, to) } else { (to, from) };
    let mut cursor = start;
    let mut years = 0;
    let mut months = 0;
    while let Some(next) = cursor.add_months(12) {
        if next <= end {
            years += 1;
            cursor = next;
        } else {
            break;
        }
    }
    while let Some(next) = cursor.add_months(1) {
        if next <= end {
            months += 1;
            cursor = next;
        } else {
            break;
        }
    }
    let remaining = days_from_civil(end.year, end.month, end.day)
        - days_from_civil(cursor.year, cursor.month, cursor.day);
    let weeks = (remaining / 7) as i32;
    let days = (remaining % 7) as i32;
    let total = days_from_civil(end.year, end.month, end.day)
        - days_from_civil(start.year, start.month, start.day);
    (years, months, weeks, days, total)
}

pub fn date_parts(years: i32, months: i32, weeks: i32, days: i32) -> String {
    if years == 0 && months == 0 && weeks == 0 && days == 0 {
        return "Same dates".to_string();
    }
    let mut parts = Vec::new();
    for (value, singular, plural) in [
        (years, "year", "years"),
        (months, "month", "months"),
        (weeks, "week", "weeks"),
        (days, "day", "days"),
    ] {
        if value != 0 {
            parts.push(format!(
                "{} {}",
                value,
                if value == 1 { singular } else { plural }
            ));
        }
    }
    parts.join(", ")
}

pub fn shift_date(from: Date, years: i32, months: i32, days: i32, add: bool) -> Option<Date> {
    let total_months = years.checked_mul(12)?.checked_add(months)?;
    let date = from.add_months(if add {
        total_months
    } else {
        total_months.checked_neg()?
    })?;
    if days == 0 {
        Some(date)
    } else {
        date.add_days(if add { days as i64 } else { -(days as i64) })
    }
}

#[cfg(test)]
mod worksheet_inverse_tests {
    use super::*;
    use crate::engine::WorksheetKind::*;

    #[test]
    fn every_target_round_trips_including_zero_interest() {
        for (kind, mut values) in [
            (Mortgage, vec![120000.0, 20000.0, 20.0, 5.0, 0.0]),
            (Mortgage, vec![120000.0, 20000.0, 20.0, 0.0, 0.0]),
            (VehicleLease, vec![30000.0, 3.0, 12.0, 15000.0, 6.0, 0.0]),
            (VehicleLease, vec![30000.0, 3.0, 12.0, 15000.0, 0.0, 0.0]),
            (FuelMpg, vec![300.0, 10.0, 0.0]),
            (FuelL100, vec![300.0, 10.0, 0.0]),
        ] {
            let def = worksheet_def(kind);
            let payment = def.default_target;
            values[payment] = solve_worksheet(kind, payment, &values).unwrap();
            for target in 0..values.len() {
                if def.non_computable.contains(&target) {
                    continue;
                }
                let mut inputs = values.clone();
                inputs[target] = f64::NAN;
                let result = solve_worksheet(kind, target, &inputs).unwrap();
                assert!(
                    (result - values[target]).abs() < 1e-6 * values[target].abs().max(1.0),
                    "{kind:?} target {target}: {result} != {}",
                    values[target]
                );
            }
        }
    }

    #[test]
    fn extreme_date_offsets_are_rejected_without_overflow() {
        let date = Date::new(2026, 1, 1).unwrap();
        assert!(shift_date(date, 0, i32::MAX, 0, true).is_none());
        assert!(shift_date(date, 0, i32::MIN, 0, false).is_none());
        assert!(date.add_days(i64::MAX).is_none());
        assert!(Date::new(0, 1, 1).is_none());
        assert!(Date::new(10000, 1, 1).is_none());
    }
}
