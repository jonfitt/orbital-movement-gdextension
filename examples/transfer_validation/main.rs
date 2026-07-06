//! Manual transfer validation runner (non-CI).
//!
//! ```text
//! cargo run --example transfer_validation
//! ```

use orbital_movement_gdextension::{run_transfer_scenario, standard_transfer_scenarios};

fn main() {
    let scenarios = standard_transfer_scenarios();
    let mut passed = 0_usize;
    let mut failed = 0_usize;

    println!(
        "Transfer validation matrix ({} scenarios)\n",
        scenarios.len()
    );

    for scenario in &scenarios {
        let report = run_transfer_scenario(scenario);
        println!("{}", report.summary_line());
        if report.passed {
            passed += 1;
        } else {
            failed += 1;
            for failure in &report.failures {
                println!("  - {failure}");
            }
        }
    }

    println!("\nResult: {passed} passed, {failed} failed");
    if failed > 0 {
        std::process::exit(1);
    }
}
