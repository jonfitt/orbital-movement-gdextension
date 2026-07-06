//! Non-CI integration tests for guided orbital transfers.
//!
//! Run manually:
//! ```text
//! cargo test transfer_validation -- --ignored --nocapture
//! ```

use orbital_movement_gdextension::{run_transfer_scenario, standard_transfer_scenarios};

#[test]
#[ignore = "manual transfer validation matrix — not run in CI"]
fn transfer_validation_matrix() {
    let mut failed = 0;
    for scenario in standard_transfer_scenarios() {
        let report = run_transfer_scenario(&scenario);
        println!("{}", report.summary_line());
        for failure in &report.failures {
            println!("  - {failure}");
        }
        if !report.passed {
            failed += 1;
        }
    }
    assert_eq!(failed, 0, "{failed} transfer scenario(s) failed");
}
