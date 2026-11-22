//! Sixth-order Krüger series in third flattening (n). Formula references:
//! https://proj.org/en/stable/operations/projections/tmerc.html
//! https://geographiclib.sourceforge.io/tm.html
use super::{Crs, Error, LocalPoint};

fn rectified(latitude: f64, delta_lon: f64, eccentricity: f64, alpha: &[f64; 6]) -> (f64, f64) {
    let tau_prime =
        (latitude.tan().asinh() - eccentricity * (eccentricity * latitude.sin()).atanh()).sinh();
    let xi_prime = tau_prime.atan2(delta_lon.cos());
    let eta_prime = (delta_lon.sin() / tau_prime.hypot(delta_lon.cos())).asinh();
    let mut xi = xi_prime;
    let mut eta = eta_prime;
    for (index, coefficient) in alpha.iter().enumerate() {
        let j = 2.0 * (index + 1) as f64;
        xi += coefficient * (j * xi_prime).sin() * (j * eta_prime).cosh();
        eta += coefficient * (j * xi_prime).cos() * (j * eta_prime).sinh();
    }
    (xi, eta)
}

pub(super) fn forward(crs: &Crs, latitude: f64, longitude: f64) -> Result<LocalPoint, Error> {
    let delta_lon = (longitude - crs.longitude_origin + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    // Avoid a plausible-looking answer near the TM singularity, and constrain
    // this implementation to its tested local projection domain. This is NOT
    // a cadastral/administrative boundary check or automatic zone selection.
    if delta_lon.abs() > 30.0_f64.to_radians() {
        return Err(Error::OutsideProjection);
    }
    let f = 1.0 / crs.ellipsoid.rf;
    let n = f / (2.0 - f);
    let n2 = n * n;
    let n3 = n2 * n;
    let n4 = n2 * n2;
    let n5 = n4 * n;
    let n6 = n3 * n3;
    let alpha = [
        n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0 - 127.0 * n5 / 288.0
            + 7891.0 * n6 / 37800.0,
        13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0 + 281.0 * n5 / 630.0
            - 1983433.0 * n6 / 1935360.0,
        61.0 * n3 / 240.0 - 103.0 * n4 / 140.0 + 15061.0 * n5 / 26880.0 + 167603.0 * n6 / 181440.0,
        49561.0 * n4 / 161280.0 - 179.0 * n5 / 168.0 + 6601661.0 * n6 / 7257600.0,
        34729.0 * n5 / 80640.0 - 3418889.0 * n6 / 1995840.0,
        212378941.0 * n6 / 319334400.0,
    ];
    let a = crs.ellipsoid.a / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
    let e = crs.ellipsoid.eccentricity_squared().sqrt();
    let (xi, eta) = rectified(latitude, delta_lon, e, &alpha);
    let (xi_zero, _) = rectified(crs.latitude_origin, 0.0, e, &alpha);
    let result = LocalPoint {
        northing_m: crs.false_northing + crs.scale * a * (xi - xi_zero),
        easting_m: crs.false_easting + crs.scale * a * eta,
    };
    if !result.northing_m.is_finite() || !result.easting_m.is_finite() {
        return Err(Error::OutsideProjection);
    }
    Ok(result)
}
