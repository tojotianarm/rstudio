use common::{config::AppConfig, error::RStudioError};

#[test]
fn configuration_rejects_invalid_sample_rate() {
    assert!(matches!(
        AppConfig { sample_rate: 0, buffer_size: 512 }.validate(),
        Err(RStudioError::InvalidConfiguration { field: "sample_rate", .. })
    ));
}

#[test]
fn configuration_rejects_invalid_buffer_size() {
    assert!(matches!(
        AppConfig { sample_rate: 44_100, buffer_size: 0 }.validate(),
        Err(RStudioError::InvalidConfiguration { field: "buffer_size", .. })
    ));
}
