use crate::diagrams::flowchart::swimlane::VARIANT;
use crate::tests::flow_golden::check_golden;

#[test]
fn swimlane_golden_cases_match() {
    check_golden("swimlane", Some(&VARIANT));
}
