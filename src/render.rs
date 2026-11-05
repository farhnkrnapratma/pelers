use crate::flowchart::{Direction, FlowchartModel, Shape};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub svg: String,
    pub width: usize,
    pub height: usize,
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn safe_value(property: &str, value: &str) -> bool {
    let value = value.trim();
    if value.is_empty()
        || value.contains(['<', '>', '{', '}', ';', '\\', '"', '\''])
        || value.to_ascii_lowercase().contains("url(")
        || value.to_ascii_lowercase().contains("expression(")
        || value.to_ascii_lowercase().contains("javascript:")
    {
        return false;
    }
    match property {
        "fill" | "stroke" | "color" => {
            let lower = value.to_ascii_lowercase();
            (lower.starts_with('#')
                && matches!(lower.len(), 4 | 5 | 7 | 9)
                && lower[1..].chars().all(|c| c.is_ascii_hexdigit()))
                || matches!(
                    lower.as_str(),
                    "none" | "black" | "white" | "red" | "green" | "blue" | "gray" | "grey"
                )
        }
        "opacity" | "fill-opacity" | "stroke-opacity" => value
            .parse::<f32>()
            .is_ok_and(|number| (0.0..=1.0).contains(&number)),
        "stroke-width" | "stroke-dashoffset" | "rx" | "ry" | "font-size" | "letter-spacing"
        | "word-spacing" => {
            let number = value.strip_suffix("px").unwrap_or(value);
            number.parse::<f32>().is_ok_and(|number| number >= 0.0)
        }
        "stroke-dasharray" => value.split_whitespace().all(|part| {
            let number = part.strip_suffix("px").unwrap_or(part);
            number.parse::<f32>().is_ok_and(|number| number >= 0.0)
        }),
        "stroke-linecap" => matches!(value, "butt" | "round" | "square"),
        "stroke-linejoin" => matches!(value, "miter" | "round" | "bevel"),
        "font-weight" => {
            matches!(value, "normal" | "bold" | "bolder" | "lighter")
                || value
                    .parse::<u16>()
                    .is_ok_and(|number| (100..=900).contains(&number) && number % 100 == 0)
        }
        "font-style" => matches!(value, "normal" | "italic" | "oblique"),
        "text-decoration" => matches!(value, "none" | "underline" | "overline" | "line-through"),
        "font-family" => {
            value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | ','))
                && value.chars().any(|c| c.is_ascii_alphabetic())
        }
        _ => false,
    }
}

fn node_styles(styles: &[String]) -> (String, String) {
    let shape_properties = [
        "fill",
        "fill-opacity",
        "stroke",
        "stroke-width",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-opacity",
        "opacity",
        "rx",
        "ry",
    ];
    let text_properties = [
        "color",
        "font-size",
        "font-family",
        "font-weight",
        "font-style",
        "text-decoration",
        "letter-spacing",
        "word-spacing",
        "opacity",
    ];
    let mut shape = BTreeMap::new();
    let mut text = BTreeMap::new();
    for declaration in styles {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        if !safe_value(&property, value) {
            continue;
        }
        if shape_properties.contains(&property.as_str()) {
            shape.insert(property.clone(), value.to_owned());
        }
        if text_properties.contains(&property.as_str()) {
            text.insert(
                if property == "color" {
                    "fill".to_owned()
                } else {
                    property
                },
                value.to_owned(),
            );
        }
    }
    let css = |properties: BTreeMap<String, String>| {
        properties
            .into_iter()
            .map(|(property, value)| format!("{property}:{value};"))
            .collect::<String>()
    };
    let shape = css(shape);
    let text = css(text);
    (
        if shape.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", escape(&shape))
        },
        if text.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", escape(&text))
        },
    )
}

pub fn render_flowchart(model: &FlowchartModel) -> Rendered {
    const NODE_WIDTH: usize = 120;
    const NODE_HEIGHT: usize = 48;
    const GAP: usize = 48;
    const PAD: usize = 24;
    let horizontal = matches!(model.direction, Direction::Lr | Direction::Rl);
    let count = model.nodes.len().max(1);
    let width = if horizontal {
        PAD * 2 + count * NODE_WIDTH + (count.saturating_sub(1)) * GAP
    } else {
        PAD * 2 + NODE_WIDTH
    };
    let height = if horizontal {
        PAD * 2 + NODE_HEIGHT
    } else {
        PAD * 2 + count * NODE_HEIGHT + (count.saturating_sub(1)) * GAP
    };

    let positions: Vec<(usize, usize)> = model
        .nodes
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let slot = if matches!(model.direction, Direction::Bt | Direction::Rl) {
                count - index - 1
            } else {
                index
            };
            if horizontal {
                (PAD + slot * (NODE_WIDTH + GAP), PAD)
            } else {
                (PAD, PAD + slot * (NODE_HEIGHT + GAP))
            }
        })
        .collect();

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}"><defs><marker id="arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto"><path d="M0,0 L10,3.5 L0,7 z"/></marker><marker id="circle" markerWidth="8" markerHeight="8" refX="4" refY="4" orient="auto"><circle cx="4" cy="4" r="3" fill="white" stroke="black"/></marker><marker id="cross" markerWidth="8" markerHeight="8" refX="4" refY="4" orient="auto"><path d="M1,1 L7,7 M7,1 L1,7" stroke="black"/></marker></defs>"#
    );
    for subgraph in &model.subgraphs {
        let members: Vec<_> = subgraph
            .nodes
            .iter()
            .filter_map(|id| {
                model
                    .nodes
                    .iter()
                    .position(|node| node.id == *id)
                    .map(|index| positions[index])
            })
            .collect();
        if let (Some((min_x, min_y)), Some((max_x, max_y))) = (
            members.iter().copied().min_by_key(|(x, _)| *x),
            members.iter().copied().max_by_key(|(x, _)| *x),
        ) {
            let top = members.iter().map(|(_, y)| *y).min().unwrap_or(min_y);
            let bottom = members.iter().map(|(_, y)| *y).max().unwrap_or(max_y);
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="none" stroke="gray" stroke-dasharray="6 4"/><text x="{}" y="{}" font-size="12">{}</text>"#,
                min_x.saturating_sub(12),
                top.saturating_sub(20),
                max_x + NODE_WIDTH - min_x + 24,
                bottom + NODE_HEIGHT - top + 32,
                min_x,
                top.saturating_sub(6),
                escape(&subgraph.label)
            ));
        }
    }
    for edge in &model.edges {
        let Some(from) = model.nodes.iter().position(|node| node.id == edge.from) else {
            continue;
        };
        let Some(to) = model.nodes.iter().position(|node| node.id == edge.to) else {
            continue;
        };
        let (fx, fy) = positions[from];
        let (tx, ty) = positions[to];
        let (x1, y1, x2, y2) = if horizontal {
            (
                fx + NODE_WIDTH,
                fy + NODE_HEIGHT / 2,
                tx,
                ty + NODE_HEIGHT / 2,
            )
        } else {
            (
                fx + NODE_WIDTH / 2,
                fy + NODE_HEIGHT,
                tx + NODE_WIDTH / 2,
                ty,
            )
        };
        let dash = if edge.marker == "-.->" {
            r#" stroke-dasharray="6 4""#
        } else {
            ""
        };
        let width = if edge.marker == "==>" {
            " stroke-width=\"3\""
        } else {
            ""
        };
        let arrow = match edge.endpoint {
            crate::flowchart::EdgeEndpoint::Circle => r#" marker-end="url(#circle)""#,
            crate::flowchart::EdgeEndpoint::Cross => r#" marker-end="url(#cross)""#,
            crate::flowchart::EdgeEndpoint::None if edge.marker == "---" => "",
            crate::flowchart::EdgeEndpoint::None => r#" marker-end="url(#arrow)""#,
        };
        svg.push_str(&format!(
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="black"{dash}{width}{arrow}/>"#
        ));
        if let Some(label) = &edge.label {
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
                (x1 + x2) / 2,
                (y1 + y2) / 2,
                escape(label)
            ));
        }
    }
    for (node, (x, y)) in model.nodes.iter().zip(&positions) {
        let (shape_style, text_style) = node_styles(&node.styles);
        let cx = x + NODE_WIDTH / 2;
        let cy = y + NODE_HEIGHT / 2;
        match node.shape {
            Shape::Rectangle => svg.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{NODE_WIDTH}" height="{NODE_HEIGHT}" fill="white" stroke="black"{shape_style}/>"#
            )),
            Shape::Rounded => svg.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{NODE_WIDTH}" height="{NODE_HEIGHT}" rx="24" fill="white" stroke="black"{shape_style}/>"#
            )),
            Shape::Diamond => svg.push_str(&format!(
                r#"<polygon points="{cx},{y} {},{cy} {cx},{} {},{cy}" fill="white" stroke="black"{shape_style}/>"#,
                x + NODE_WIDTH,
                y + NODE_HEIGHT,
                x
            )),
            Shape::Circle => svg.push_str(&format!(
                r#"<circle cx="{cx}" cy="{cy}" r="24" fill="white" stroke="black"{shape_style}/>"#
            )),
            Shape::Stadium => svg.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{NODE_WIDTH}" height="{NODE_HEIGHT}" rx="24" ry="24" fill="white" stroke="black"{shape_style}/>"#
            )),
            Shape::Cylinder => {
                let bottom = y + NODE_HEIGHT;
                svg.push_str(&format!(
                    r#"<path d="M{x} {y_plus} A60 8 0 0 1 {right} {y_plus} V{bottom_minus} A60 8 0 0 1 {x} {bottom_minus} Z" fill="white" stroke="black"{shape_style}/>"#,
                    y_plus = y + 8,
                    right = x + NODE_WIDTH,
                    bottom_minus = bottom - 8,
                ));
            }
            Shape::Hexagon => svg.push_str(&format!(
                r#"<polygon points="{},{} {},{} {},{} {},{} {},{} {},{}" fill="white" stroke="black"{shape_style}/>"#,
                x + 12,
                y,
                x + NODE_WIDTH - 12,
                y,
                x + NODE_WIDTH,
                cy,
                x + NODE_WIDTH - 12,
                y + NODE_HEIGHT,
                x + 12,
                y + NODE_HEIGHT,
                x,
                cy
            )),
        }
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{cy}" text-anchor="middle" dominant-baseline="middle"{text_style}>{}</text>"#,
            escape(&node.label)
        ));
    }
    svg.push_str("</svg>");
    Rendered { svg, width, height }
}

#[cfg(test)]
mod tests {
    use super::render_flowchart;
    use crate::flowchart::parse;

    #[test]
    fn renders_nodes_edges_and_escaped_labels() {
        let model = parse("flowchart LR\n  A[<Start>] --> B[End & done]").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("<line"));
        assert!(rendered.svg.contains("&lt;Start&gt;"));
        assert!(rendered.svg.contains("End &amp; done"));
        assert!(rendered.svg.contains("marker-end"));
    }

    #[test]
    fn direction_changes_aspect_ratio() {
        let model = parse("flowchart TD\n  A --> B").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.height > rendered.width);
    }

    #[test]
    fn renders_shape_elements() {
        let model = parse("flowchart LR\n  A(Start) --> B{Choice}").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("rx=\"24\""));
        assert!(rendered.svg.contains("<polygon"));
    }

    #[test]
    fn renders_extended_shape_geometry() {
        let model = parse(
            "flowchart LR\n  circle((Circle)) --> stadium([Stadium])\n  stadium --> cylinder[(Cylinder)]\n  cylinder --> hexagon{{Hexagon}}",
        )
        .unwrap();
        let svg = render_flowchart(&model).svg;
        assert!(svg.contains("<circle"));
        assert!(svg.contains("rx=\"24\" ry=\"24\""));
        assert!(svg.contains("<path d=\"M"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn renders_escaped_edge_labels() {
        let model = parse("flowchart LR\n  A -->|<yes>| B").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("&lt;yes&gt;"));
    }

    #[test]
    fn renders_edge_marker_styles() {
        let model = parse("flowchart LR\n  A -.-> B\n  B ==> C\n  C --- D").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("stroke-dasharray"));
        assert!(rendered.svg.contains("stroke-width=\"3\""));
        assert_eq!(rendered.svg.matches("marker-end").count(), 2);
    }

    #[test]
    fn renders_circle_and_cross_endpoints() {
        let model = parse("flowchart LR\n  A --o B\n  B --x C").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("id=\"circle\""));
        assert!(rendered.svg.contains("id=\"cross\""));
        assert!(rendered.svg.contains("marker-end=\"url(#circle)\""));
        assert!(rendered.svg.contains("marker-end=\"url(#cross)\""));
    }

    #[test]
    fn renders_subgraph_boundary() {
        let model = parse("flowchart LR\n  subgraph Group\n    A --> B\n  end").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("stroke-dasharray"));
        assert!(rendered.svg.contains(">Group</text>"));
    }

    #[test]
    fn applies_allowlisted_styles_and_last_declaration_wins() {
        let model = parse(
            "flowchart LR\n  A[Styled]\n  style A fill:#abc,stroke-width:2,color:red\n  style A fill:#def",
        )
        .unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("fill:#def"));
        assert!(!rendered.svg.contains("fill:#abc"));
        assert!(rendered.svg.contains("stroke-width:2"));
        assert!(rendered.svg.contains("style=\"fill:red;\""));
    }

    #[test]
    fn ignores_malformed_and_unsafe_style_values() {
        let model = parse(
            "flowchart LR\n  A[Styled]\n  style A fill:url(javascript:alert(1)),stroke:<red>,opacity:2,unknown:value",
        )
        .unwrap();
        let rendered = render_flowchart(&model);
        assert!(!rendered.svg.contains("javascript"));
        assert!(!rendered.svg.contains("unknown"));
        assert!(!rendered.svg.contains("stroke:&lt;red&gt;"));
        assert!(!rendered.svg.contains("opacity:2"));
    }

    #[test]
    fn escapes_style_attribute_values() {
        let model =
            parse("flowchart LR\n  A[Styled]\n  style A font-family:Arial,fill:#fff").unwrap();
        let rendered = render_flowchart(&model);
        assert!(rendered.svg.contains("font-family:Arial"));
        assert!(!rendered.svg.contains("<style"));
    }
}
