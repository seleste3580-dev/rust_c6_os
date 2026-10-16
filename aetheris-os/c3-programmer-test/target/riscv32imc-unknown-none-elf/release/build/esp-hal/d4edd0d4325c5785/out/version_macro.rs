
#[doc(hidden)]
#[macro_export]
macro_rules! __esp_hal_at_least_version {
    ((0, $_minor:literal, $_patch:literal) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    ((1, 0, $_patch:literal) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    ((1, 1, $_patch:literal) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    ((1, 2, 0) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    ((1, 2, 1) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    ((1, 2, 2) => { $($selected:tt)* } $($rest:tt)*) => {
        $($selected)*
    };
    (($major:literal, $minor:literal, $patch:literal) => { $($skipped:tt)* } $($rest:tt)*) => {
        $crate::__esp_hal_at_least_version! { $($rest)* }
    };
    (, $($rest:tt)*) => {
        $crate::__esp_hal_at_least_version! { $($rest)* }
    };
    (_ => { $($fallback:tt)* } $(,)?) => {
        $($fallback)*
    };
}
