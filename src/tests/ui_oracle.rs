use std::fs;

use crate::ui::{
    ALONE_STYLE, BAR_STYLE, BUTTON_STYLE, GAP, Icon, OVER_STYLE, PAN, STEP, Size, Thrown, frame,
    message_of, room_within, summary, zoom_icon_markup,
};
use serde_json::Value as Json;

fn load() -> Json {
    serde_json::from_str(&fs::read_to_string(crate::tests::fixture_path("ui.json")).unwrap())
        .unwrap()
}

fn number(value: &Json) -> f64 {
    value.as_f64().unwrap()
}

#[test]
fn frames_match_the_oracle() {
    let fixtures = load();
    for case in fixtures["frames"].as_array().unwrap() {
        let size = |key: &str| Size {
            width: number(&case[key]["width"]),
            height: number(&case[key]["height"]),
        };
        let got = frame(
            size("natural"),
            size("box"),
            number(&case["scale"]),
            number(&case["x"]),
            number(&case["y"]),
        );
        let want = &case["frame"];
        assert_eq!(got.x, number(&want["x"]), "{case}");
        assert_eq!(got.y, number(&want["y"]), "{case}");
        assert_eq!(got.width, number(&want["width"]), "{case}");
        assert_eq!(got.height, number(&want["height"]), "{case}");
    }
}

#[test]
fn summaries_match_the_oracle() {
    let fixtures = load();
    for case in fixtures["messages"].as_array().unwrap() {
        assert_eq!(
            summary(case["text"].as_str().unwrap()),
            case["summary"].as_str().unwrap()
        );
    }
}

#[test]
fn messages_match_the_oracle() {
    let fixtures = load();
    for case in fixtures["messageOfs"].as_array().unwrap() {
        let error = &case["error"];
        let thrown = if let Some(message) = error.get("message") {
            Thrown::Error(message.as_str().unwrap().to_owned())
        } else if let Some(text) = error.as_str() {
            if text == "__undefined__" {
                Thrown::Undefined
            } else {
                Thrown::Text(text.to_owned())
            }
        } else if let Some(number) = error.as_f64() {
            Thrown::Number(number)
        } else {
            Thrown::Null
        };
        assert_eq!(message_of(&thrown), case["text"].as_str().unwrap());
    }
}

#[test]
fn rooms_match_the_oracle() {
    let fixtures = load();
    for case in fixtures["rooms"].as_array().unwrap() {
        let left = case["left"].as_f64().unwrap_or(0.0);
        let right = case["right"].as_f64().unwrap_or(0.0);
        assert_eq!(
            room_within(number(&case["clientWidth"]), left, right),
            number(&case["room"])
        );
    }
}

#[test]
fn constants_match_the_oracle() {
    let constants = &load()["constants"];
    assert_eq!(STEP, number(&constants["STEP"]));
    assert_eq!(PAN, number(&constants["PAN"]));
    assert_eq!(GAP, number(&constants["GAP"]));
    assert_eq!(BAR_STYLE, constants["BAR_STYLE"].as_str().unwrap());
    assert_eq!(BUTTON_STYLE, constants["BUTTON_STYLE"].as_str().unwrap());
    assert_eq!(OVER_STYLE, constants["OVER_STYLE"].as_str().unwrap());
    assert_eq!(ALONE_STYLE, constants["ALONE_STYLE"].as_str().unwrap());
    assert_eq!(
        Icon::ZoomIn.path(),
        constants["ICONS"]["zoomIn"].as_str().unwrap()
    );
    assert_eq!(
        Icon::ZoomOut.path(),
        constants["ICONS"]["zoomOut"].as_str().unwrap()
    );
    assert_eq!(
        Icon::Reset.path(),
        constants["ICONS"]["reset"].as_str().unwrap()
    );
    assert_eq!(
        Icon::ZoomIn.default_label(),
        constants["LABELS"]["zoomIn"].as_str().unwrap()
    );
    assert_eq!(
        Icon::ZoomOut.default_label(),
        constants["LABELS"]["zoomOut"].as_str().unwrap()
    );
    assert_eq!(
        Icon::Reset.default_label(),
        constants["LABELS"]["reset"].as_str().unwrap()
    );
    assert!(zoom_icon_markup(Icon::Reset).contains(Icon::Reset.path()));
}
