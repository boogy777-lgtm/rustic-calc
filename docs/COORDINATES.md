# WGS84 → MSK: templates and custom coordinate systems

Open **View → WGS84 → MSK**, press **Ctrl+G**, or start with
`RusticCalc.exe --mode coordinates` (`wgs84` and `msk` are aliases).
The first ten modes and their shortcuts are unchanged. The new mode is offline
and adds no runtime dependency, network request, filesystem or registry access.

## Use

1. Select a region and the required **system and zone**. A regional name does
   not uniquely identify a CRS. Zones are selected manually, not inferred from
   the input longitude or administrative boundaries.
2. Edit the populated **PROJ parameters**, or choose **Custom parameters** and
   paste a supported definition. Parameters are not locked to bundled templates.
3. Enter **latitude, longitude in decimal degrees**, and **WGS84 ellipsoidal
   height h in metres**. Coordinate fields accept a dot/comma decimal separator.
   The visible initial height `0` is an assumption, not a measured height. It is
   not a height above sea level. Blank/invalid values are never silently zeroed.
4. Press Enter or Calculate. Outputs are **X = Northing**, **Y = Easting**, in
   metres, in selectable read-only native fields. Native copy/paste and
   Tab/Shift+Tab remain available. F2 opens/returns from help; Esc closes help.

Each template and each region's custom definition has its own in-memory draft.
Switching templates, regions, modes, language or DPI preserves edits. Reset
restores only the selected template (or clears the selected custom draft).
**Drafts are not saved across application restarts.** Copy the PROJ text to keep
it. The calculator does not silently create configuration files.

## Supported PROJ subset

Example: the bundled MSK-50 zone 2 definition:

```text
+proj=tmerc +lat_0=0 +lon_0=38.48333333333 +k=1 +x_0=2250000 +y_0=-5712900.566 +ellps=krass +towgs84=23.57,-140.95,-79.8,0,0.35,0.79,-0.22 +units=m +no_defs
```

Required projection values are explicit: `proj=tmerc`, `lat_0`, `lon_0`, `k`
or `k_0`, `x_0`, `y_0`, and `units=m`. Ellipsoid: `ellps=krass`, `bessel`,
`WGS84`, `GRS80`, or custom `a` and `rf`. The datum must be explicit: a 3- or
7-value `towgs84`, or `datum=WGS84` with the WGS84 ellipsoid. An ellipsoid alone
does not specify a datum. `no_defs` and `type=crs` are accepted metadata.

PROJ numbers use decimal dots; the 7-value order is:
`dx,dy,dz,rx,ry,rz,scale`. Translations are metres, rotations arcseconds and
scale ppm. `towgs84` describes **local datum → WGS84**, using the legacy
position-vector convention. This mode applies the inverse small-angle operation
for WGS84 → local datum, compatible with legacy PROJ/proj4.js definitions.
Do not directly paste MapInfo datum arrays: rotation signs can differ.

In PROJ, **x_0 means false easting and y_0 means false northing**. Those are not
the surveying X/Y output order. Multiline definitions and spaces after commas
in `towgs84` are accepted. Unknown, duplicate, conflicting, incomplete,
nonfinite and unsupported parameters fail visibly; no partial interpretation.
This is **not a complete PROJ/WKT parser**. WKT, EPSG identifiers, other
projections, `nadgrids`, arbitrary axis/prime-meridian/unit changes, datum epochs
and transformations requiring grids are not supported.

Validation bounds: latitude strictly between −90° and 90°, longitude −180° to
180°, height −10000 to 100000 m; local `lat_0` strictly between −90° and 90°,
`lon_0` −360° to 360° (wrapped for Chukotka), 0 < k ≤ 2, offsets within ±1e9 m,
6e6 ≤ a ≤ 7e6 m, 200 ≤ rf ≤ 400. Datum translations are limited to ±1e6 m,
rotations ±60 arcseconds and scale ±10000 ppm. Numeric syntax is intentionally
bounded to ordinary PROJ decimal literals; a `+` within a numeric literal is
not accepted as another sign/exponent separator.

## Catalogue coverage and provenance

The bundled catalogue contains **262 definitions for 79 regional groups** from
[geo-ts/msk](https://github.com/geo-ts/msk), pinned at revision
`946049d80f0f4b4956a53383961c8c6f4431169f` (2024-07-27), **Apache-2.0**.
The application remains MIT; upstream data retain their license. Full terms
and source revision are embedded in F2 help, including in the single portable
EXE. See [NOTICE.md](../NOTICE.md) and
[the upstream license](../third_party/geo-ts-msk-LICENSE.txt).

There are 89 selectable regional groups. No built-in definition is currently
available for Tuva, Khakassia, Jewish Autonomous Oblast, Yamalo-Nenets Autonomous
Okrug, Crimea, Sevastopol, Donetsk, Luhansk, Zaporizhzhia and Kherson. Those
groups explicitly request a custom definition. Regional group labels do not
infer parameters or make statements about boundaries. Krasnoyarsk city/local
systems are grouped with Krasnoyarsk Krai; numeric definitions are unchanged.

Reproduce the catalogue (developer-only network operation; not run at startup):

```text
python tools/import_msk_templates.py
```

The importer parses JSON data from the pinned TS file, never executes it, and
verifies these SHA-256 hashes before writing the data/license:

| File | SHA-256 |
| --- | --- |
| index.ts | 4862fc4121c2afd0919b7a0349dbc2cfb0264a82a8e4263da7610b28b59b4cad |
| LICENSE | 43070e2d4e532684de521b885f385d0841030efa2b1a20bafb76133a5e1379c1 |

Other public references consulted, not copied as software/resources:

- [TerrainGIS, MSK-46](https://terraingis.ru/msk-46.html): confirms the same
  zone definitions and distinguishes MapInfo/PROJ rotation signs.
- [GeoProf regional list](https://geoprof.nethouse.ru/parametry-mestnyh-sistem-koordinat):
  a secondary catalogue. Some datum values differ; it was **not** silently merged
  into the vendored catalogue or treated as an official authority.
- [PROJ Transverse Mercator formulas](https://proj.org/en/stable/operations/projections/tmerc.html)
  and [GeographicLib TM notes](https://geographiclib.sourceforge.io/tm.html).

## Mathematical implementation and accuracy limits

`src/coordinates/` implements WGS84 geodetic → ECEF, inverse 3/7-parameter
datum operation, local ellipsoidal latitude/longitude and a sixth-order Krüger
Transverse Mercator series. A ±30° central-meridian longitude limit prevents
out-of-domain calculations; it does **not** validate a region or cadastral zone.
No geoid/vertical datum conversion, grid shift, epoch, plate-velocity correction,
automatic zone selection or surveying localization is provided.

**Bundled definitions are community templates, not official/certified survey
parameters.** They may be wrong, outdated, or not applicable to a particular
local realization. Do not use the output for legal/cadastral work without
official parameters and independent surveyed control points. Three decimal
places are formatting, not a promise of millimetre positional accuracy.

## Regression evidence

- Strict parsing, axis order, ellipsoid/datum conflicts, nonfinite/boundary
  inputs, nonzero latitude origin, date-line wrap and output invalidation.
- All 262 definitions parse and have numerical fixtures: **786 synthetic
  reference points** generated independently with proj4.js 2.12.1, including
  45°/60°/80° latitudes, offsets from the central meridian and nonzero heights.
  Six additional custom-CRS fixtures cover a 3-value datum shift, GRS80, custom
  ellipsoids, scale, nonzero origin, date-line wrap and numerical domain bounds.
  For explicit all-zero shifts, the reference generator adds a non-built-in
  datum name **only to proj4.js's input** to avoid its PJD_NODATUM shortcut;
  all ellipsoid/numeric values and tested definitions remain unchanged. This
  makes the reference apply the requested ellipsoid change rather than skipping it.
  `cargo test --locked` compares both output components within 0.00001 m.
  The extra GRS80 zero-shift case permits 0.0002 m: proj4.js deliberately
  [considers GRS80/WGS84 identical](https://cdn.jsdelivr.net/npm/proj4@2.12.1/lib/datumUtils.js)
  for datum comparison, whereas this implementation retains the explicit
  ellipsoid/ECEF conversion. Other fixture tolerances are unchanged.
  These synthetic points test algorithms, **not** regional applicability or
  geodetic accuracy. The upstream Omsk example is tested separately.
- Native hidden-window tests cover read-only outputs, multiline editing state,
  embedded full license, language/DPI recreation and per-template drafts.
- `python tools/smoke_coordinates.py target/x86_64-pc-windows-msvc/release/calc1.exe`
  launches only test-owned live windows and checks native text replacement,
  Enter/Tab, clearing stale results, edited parameters, missing-template regions,
  mode/locale/theme transitions and F2/Esc/full-license access in both locales
  and light/dark themes. It does not inject global keyboard events or touch the
  clipboard. Optional Pillow captures filled forms; it is not a runtime dependency.
- Layout/text measurements cover RU/EN and 72/96/120/144/168/192/240/288/384 DPI.
  Hidden/synthetic tests are not physical mixed-DPI monitor movement, screen
  reader or High Contrast validation.

To reproduce reference fixtures, obtain the official
[`proj4.js 2.12.1` distribution](https://cdn.jsdelivr.net/npm/proj4@2.12.1/dist/proj4.js)
into a local development-only file, then run:

```text
node tools/coordinate_reference.cjs <local-proj4.js-path>
```

The generator verifies SHA-256
`9ecae61d88a976481c70d25069066205b10d541f098cdb69c02f3dd4dcac99cf`
before loading the reference library. There is no npm installation or new
application runtime dependency. Only numerical fixtures are committed.
