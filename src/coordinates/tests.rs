use super::*;

const SIMPLE: &str = "+proj=tmerc +lat_0=0 +lon_0=39 +k=1 +x_0=500000 +y_0=0 +ellps=WGS84 +datum=WGS84 +units=m +no_defs";

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} (tolerance {tolerance})"
    );
}

#[test]
fn origin_and_axis_order_are_explicit() {
    let crs = Crs::parse(SIMPLE).unwrap();
    let result = crs.convert(Wgs84::parse("0", "39", "0").unwrap()).unwrap();
    near(result.northing_m, 0.0, 1e-9);
    near(result.easting_m, 500000.0, 1e-9);
    let east = crs.convert(Wgs84::parse("0", "40", "0").unwrap()).unwrap();
    near(east.easting_m, 611325.1809637867, 1e-6);
    near(east.northing_m, 0.0, 1e-9);
    let north = crs.convert(Wgs84::parse("1", "39", "0").unwrap()).unwrap();
    near(north.northing_m, 110574.38855779878, 1e-6);
    near(north.easting_m, 500000.0, 1e-9);
}

#[test]
fn upstream_omsk_control_point_matches_without_axis_swap() {
    let template = catalogue::all().find(|t| t.id == "MSK55z2").unwrap();
    let point = Wgs84::parse("54.985876655", "73.356141837", "0").unwrap();
    let result = Crs::parse(template.definition)
        .unwrap()
        .convert(point)
        .unwrap();
    // Upstream proj4.js fixture, metres; numerical agreement is not a survey.
    near(result.easting_m, 2161894.502199385, 0.002);
    near(result.northing_m, 483672.32940373477, 0.002);
}

#[test]
fn every_vendored_template_parses_and_projects_near_its_central_meridian() {
    let mut ids = std::collections::BTreeSet::new();
    let mut regions = std::collections::BTreeSet::new();
    for template in catalogue::all() {
        assert!(ids.insert(template.id), "duplicate {}", template.id);
        regions.insert(template.region);
        let crs =
            Crs::parse(template.definition).unwrap_or_else(|e| panic!("{}: {e:?}", template.id));
        let lon = (crs.longitude_origin.to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            crs.convert(Wgs84 {
                latitude_deg: 60.0,
                longitude_deg: lon,
                height_m: 0.0
            })
            .is_ok(),
            "{}",
            template.id
        );
    }
    assert_eq!(ids.len(), 262);
    assert_eq!(regions.len(), 79);
    assert!(catalogue::LICENSE.contains("Apache License"));
}

#[test]
fn nonzero_latitude_origin_and_longitude_wrap_work() {
    let definition = SIMPLE
        .replace("lat_0=0", "lat_0=55")
        .replace("lon_0=39", "lon_0=186.45");
    let result = Crs::parse(&definition)
        .unwrap()
        .convert(Wgs84::parse("55", "-173.55", "0").unwrap())
        .unwrap();
    near(result.easting_m, 500000.0, 1e-7);
    near(result.northing_m, 0.0, 1e-7);
}

#[test]
fn finite_decimal_coordinates_are_required_including_height() {
    assert!(Wgs84::parse("55,7", "37,6", "150,5").is_ok());
    for text in ["", "NaN", "inf", "1e999", "55°45'", "1 234", "1,2.3"] {
        assert!(Wgs84::parse(text, "37", "0").is_err());
        assert!(Wgs84::parse("55", text, "0").is_err());
        assert!(Wgs84::parse("55", "37", text).is_err());
    }
    for latitude in ["-90", "90", "91"] {
        assert!(Wgs84::parse(latitude, "0", "0").is_err());
    }
    for longitude in ["-180.1", "181"] {
        assert!(Wgs84::parse("0", longitude, "0").is_err());
    }
    for height in ["-10001", "100001"] {
        assert!(Wgs84::parse("0", "0", height).is_err());
    }
    assert!(Crs::parse(SIMPLE)
        .unwrap()
        .convert(Wgs84 {
            latitude_deg: f64::NAN,
            longitude_deg: 0.0,
            height_m: 0.0
        })
        .is_err());
}

#[test]
fn missing_datum_and_unsupported_or_conflicting_parameters_never_fall_back() {
    assert_eq!(Crs::parse("").unwrap_err(), Error::EmptyDefinition);
    assert_eq!(
        Crs::parse(&SIMPLE.replace("+datum=WGS84", "")).unwrap_err(),
        Error::DatumRequired
    );
    for suffix in [
        "+nadgrids=missing.gsb",
        "+axis=neu",
        "+init=epsg:4326",
        "+foo=1",
        "+lon_0=40",
        "+k_0=1",
        "+towgs84=0,0,0",
        "+a=6378137",
    ] {
        assert!(
            Crs::parse(&format!("{SIMPLE} {suffix}")).is_err(),
            "{suffix}"
        );
    }
    for (from, to) in [
        ("proj=tmerc", "proj=merc"),
        ("units=m", "units=ft"),
        ("ellps=WGS84", "ellps=unknown"),
        ("ellps=WGS84", "ellps=krass"),
        ("k=1", "k=0"),
        ("k=1", "k=NaN"),
        ("x_0=500000", "x_0=inf"),
    ] {
        assert!(Crs::parse(&SIMPLE.replace(from, to)).is_err(), "{to}");
    }
}

#[test]
fn three_and_seven_parameter_shifts_and_custom_ellipsoids_are_supported() {
    let three = SIMPLE.replace(
        "+ellps=WGS84 +datum=WGS84",
        "+a=6378245 +rf=298.3 +towgs84=23.57,-140.95,-79.8",
    );
    assert!(Crs::parse(&three).is_ok());
    let seven = three.replace(
        "23.57,-140.95,-79.8",
        "23.57, -140.95, -79.8, 0, 0.35, 0.79, -0.22",
    );
    assert!(Crs::parse(&seven.replace(" +", "\r\n+")).is_ok());
    for values in [
        "",
        "0,0",
        "0,0,0,0",
        "0,0,0,0,0,0,0,0",
        "0,0,inf",
        "0,0,0,61,0,0,0",
    ] {
        assert!(
            Crs::parse(&three.replace("23.57,-140.95,-79.8", values)).is_err(),
            "{values}"
        );
    }
}

#[test]
fn far_away_points_are_rejected_but_not_misrepresented_as_region_validation() {
    assert_eq!(
        Crs::parse(SIMPLE)
            .unwrap()
            .convert(Wgs84::parse("55", "120", "0").unwrap())
            .unwrap_err(),
        Error::OutsideProjection
    );
}

#[test]
fn all_templates_match_independent_proj4js_reference_points() {
    let templates: std::collections::BTreeMap<_, _> = catalogue::all().map(|t| (t.id, t)).collect();
    let mut count = 0;
    let mut maximum_error = 0.0_f64;
    for line in include_str!("../../data/msk_reference.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 6);
        let point = Wgs84::parse(fields[1], fields[2], fields[3]).unwrap();
        let projected = Crs::parse(templates[fields[0]].definition)
            .unwrap()
            .convert(point)
            .unwrap();
        let northing = fields[4].parse::<f64>().unwrap();
        let easting = fields[5].parse::<f64>().unwrap();
        let error = (projected.northing_m - northing)
            .abs()
            .max((projected.easting_m - easting).abs());
        assert!(
            error <= 0.00001,
            "{} / {} / {}: error {error} m",
            fields[0],
            fields[1],
            fields[2]
        );
        maximum_error = maximum_error.max(error);
        count += 1;
    }
    assert_eq!(count, 786);
    println!("{count} proj4.js reference points; max component error {maximum_error:.9} m");
}

#[test]
fn custom_parameters_match_independent_reference_including_three_value_datum_and_bounds() {
    let mut count = 0;
    for line in include_str!("../../data/custom_coordinate_reference.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 7);
        let point = Wgs84::parse(fields[2], fields[3], fields[4]).unwrap();
        let projected = Crs::parse(fields[1]).unwrap().convert(point).unwrap();
        let northing = fields[5].parse::<f64>().unwrap();
        let easting = fields[6].parse::<f64>().unwrap();
        let error = (projected.northing_m - northing)
            .abs()
            .max((projected.easting_m - easting).abs());
        // proj4.js compareDatums deliberately treats WGS84 and GRS80 as
        // identical (eccentricity-squared tolerance 5e-11), skipping this tiny
        // ellipsoid change. We retain the explicit ellipsoid/ECEF conversion.
        // Source: proj4@2.12.1/lib/datumUtils.js. Do not loosen other fixtures.
        let tolerance = if fields[0] == "grs80-zero-shift" {
            0.0002
        } else {
            0.00001
        };
        assert!(
            error <= tolerance,
            "{}: {error} m > {tolerance} m",
            fields[0]
        );
        println!("{}: reference component error {error:.9} m", fields[0]);
        count += 1;
    }
    assert_eq!(count, 6);
}
