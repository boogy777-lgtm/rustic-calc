use rustic_calc_app::units::Date;

/// The Slint edition currently targets the same Windows x64 environment as the
/// packaged application. Read the user's local calendar date without invoking
/// a subprocess or adding a date/time runtime dependency.
pub fn local_date() -> Date {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: GetLocalTime writes one SYSTEMTIME into the valid mutable pointer.
    unsafe { GetLocalTime(&mut local) };
    Date::new(
        i32::from(local.wYear),
        u32::from(local.wMonth),
        u32::from(local.wDay),
    )
    .expect("Windows returned an invalid local date")
}
