//! Integration tests for global config editor.

use std::env;

#[test]
fn init_and_set_global_config_under_temp_storage() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    env::set_var(
        "GLEAN_STORAGE_ROOT",
        storage_tmp.path().to_string_lossy().as_ref(),
    );

    let path = glean_host::config::init_global_config(true).expect("init");
    assert!(path.is_file());

    let merged = glean_host::config::load_merged_config().expect("load");
    assert_eq!(merged.log.level, "info");

    glean_host::config::set_global_key("log.level".into(), "debug".into()).expect("set");
    let merged2 = glean_host::config::load_merged_config().expect("load2");
    assert_eq!(merged2.log.level, "debug");
}
