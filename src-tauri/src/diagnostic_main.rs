fn main() {
    assert!(
        std::env::var_os("KEYMOUSE_DIAGNOSTIC_DATA_DIR").is_some(),
        "Diagnostic runs require an isolated data directory"
    );
    keymouse_lib::run();
}
