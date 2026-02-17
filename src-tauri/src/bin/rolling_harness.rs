use std::env;
use std::fs;
use std::process::ExitCode;

use spittle_app_lib::rolling_harness::{
    normalize_scenario, replay_hypotheses, ReplayScenario, RewriteStrategy,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: cargo run -p spittle --bin rolling_harness -- <scenario_json_path>");
        return ExitCode::from(2);
    };

    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read scenario file '{}': {}", path, e);
            return ExitCode::from(1);
        }
    };

    let scenario: ReplayScenario = match serde_json::from_str(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Failed to parse scenario JSON '{}': {}. Expected {{\"name\": \"...\", \"hypotheses\": [\"...\", ...]}}",
                path, e
            );
            return ExitCode::from(1);
        }
    };
    let scenario = normalize_scenario(scenario);
    if scenario.hypotheses.is_empty() {
        eprintln!("Scenario '{}' contains no hypotheses", scenario.name);
        return ExitCode::from(1);
    }

    let perfect = replay_hypotheses(&scenario.hypotheses, RewriteStrategy::Perfect);
    let under_delete_1 = replay_hypotheses(
        &scenario.hypotheses,
        RewriteStrategy::UnderDeletePerRewrite(1),
    );

    println!("Rolling Harness Report");
    println!("scenario: {}", scenario.name);
    println!("file: {}", path);
    println!("hypotheses: {}", perfect.hypotheses_count);
    println!("rewrites: {}", perfect.rewrites_applied);
    println!(
        "expected-final-chars: {}",
        perfect.final_expected.chars().count()
    );
    println!();
    println!(
        "perfect-rewrite: {}",
        if perfect.matches_expected {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "under-delete(+1/rewrite) drift: {}",
        if under_delete_1.matches_expected {
            "NO"
        } else {
            "YES"
        }
    );

    if !under_delete_1.matches_expected {
        println!(
            "under-delete-final-prefix: {}",
            &under_delete_1
                .final_actual
                .chars()
                .take(40)
                .collect::<String>()
        );
    }

    ExitCode::SUCCESS
}
