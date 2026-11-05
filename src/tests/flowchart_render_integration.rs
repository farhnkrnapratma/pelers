use crate::{Direction, EdgeEndpoint, Shape, parse_flowchart, render_flowchart};

#[test]
fn public_parser_and_renderer_produce_a_connected_flowchart() {
    let model = parse_flowchart(
        "flowchart LR\n  start[Start] -->|continue| decision{Ready?}\n  decision --> finish(Complete)",
    )
    .expect("valid flowchart should parse");

    assert_eq!(model.direction, Direction::Lr);
    assert_eq!(model.nodes.len(), 3);
    assert_eq!(model.edges.len(), 2);
    assert_eq!(model.edges[0].label.as_deref(), Some("continue"));

    let rendered = render_flowchart(&model);
    assert!(rendered.svg.starts_with("<svg "));
    assert!(rendered.svg.contains(">Start</text>"));
    assert!(rendered.svg.contains(">Complete</text>"));
    assert!(rendered.svg.contains("marker-end=\"url(#arrow)\""));
    assert_eq!(rendered.width, 2 * 24 + 3 * 120 + 2 * 48);
    assert_eq!(rendered.height, 2 * 24 + 48);
}

#[test]
fn renderer_escapes_node_and_edge_text_as_svg_data() {
    let model = parse_flowchart(
        "flowchart LR\n  source[<Start & \"quoted\">] -->|<yes & 'go'>| target[Done]",
    )
    .expect("valid flowchart should parse");

    let svg = render_flowchart(&model).svg;

    assert!(svg.contains("&lt;Start &amp; &quot;quoted&quot;&gt;"));
    assert!(svg.contains("&lt;yes &amp; &apos;go&apos;&gt;"));
    assert!(!svg.contains("<Start &"));
    assert!(!svg.contains("<yes &"));
}

#[test]
fn directions_change_layout_orientation_and_reverse_order() {
    let vertical = render_flowchart(
        &parse_flowchart("flowchart TB\n  top[Top] --> bottom[Bottom]")
            .expect("TB flowchart should parse"),
    );
    let horizontal = render_flowchart(
        &parse_flowchart("flowchart LR\n  left[Left] --> right[Right]")
            .expect("LR flowchart should parse"),
    );
    let reversed = render_flowchart(
        &parse_flowchart("flowchart RL\n  left[Left] --> right[Right]")
            .expect("RL flowchart should parse"),
    );

    assert!(vertical.height > vertical.width);
    assert!(horizontal.width > horizontal.height);
    assert!(reversed.width > reversed.height);
    assert!(reversed.svg.contains(">Left</text>"));
    assert!(reversed.svg.contains(">Right</text>"));
    assert!(reversed.svg.contains("x1=\"312\""));
}

#[test]
fn shapes_and_edge_markers_are_visible_in_rendered_svg() {
    let model = parse_flowchart(
        "flowchart LR\n  rectangle[Rectangle] --> rounded(Rounded)\n  rounded -.-> diamond{Diamond}\n  diamond ==> plain\n  plain --- end",
    )
    .expect("valid flowchart should parse");

    assert_eq!(model.nodes[0].shape, Shape::Rectangle);
    assert_eq!(model.nodes[1].shape, Shape::Rounded);
    assert_eq!(model.nodes[2].shape, Shape::Diamond);

    let svg = render_flowchart(&model).svg;
    assert!(svg.contains("rx=\"24\""));
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains("stroke-width=\"3\""));
    assert_eq!(svg.matches("marker-end=\"url(#arrow)\"").count(), 3);
}

#[test]
fn extended_mermaid_shapes_keep_fixed_node_geometry() {
    let model = parse_flowchart(
        "flowchart LR\n  circle((Circle)) --> stadium([Stadium])\n  stadium --> cylinder[(Cylinder)]\n  cylinder --> hexagon{{Hexagon}}",
    )
    .expect("extended shapes should parse");

    let rendered = render_flowchart(&model);
    assert_eq!(rendered.width, 2 * 24 + 4 * 120 + 3 * 48);
    assert_eq!(rendered.height, 2 * 24 + 48);
    assert!(rendered.svg.contains("<circle"));
    assert!(rendered.svg.contains("<path d=\"M"));
    assert!(rendered.svg.contains(">Hexagon</text>"));
}

#[test]
fn subgraph_metadata_and_boundary_are_preserved() {
    let model = parse_flowchart(
        "flowchart TB\n  subgraph Processing\n    input[Input] --> output[Output]\n  end",
    )
    .expect("valid subgraph should parse");

    let subgraph = model.subgraphs.first().expect("subgraph should be present");
    assert_eq!(subgraph.id, "Processing");
    assert_eq!(subgraph.label, "Processing");
    assert_eq!(subgraph.nodes, ["input", "output"]);

    let svg = render_flowchart(&model).svg;
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains(">Processing</text>"));
}

#[test]
fn circle_and_cross_endpoints_are_preserved_and_rendered() {
    let model = parse_flowchart("flowchart LR\n  source --o middle\n  middle --x target")
        .expect("endpoint syntax should parse");

    assert_eq!(model.edges[0].endpoint, EdgeEndpoint::Circle);
    assert_eq!(model.edges[1].endpoint, EdgeEndpoint::Cross);

    let svg = render_flowchart(&model).svg;
    assert!(svg.contains("marker-end=\"url(#circle)\""));
    assert!(svg.contains("marker-end=\"url(#cross)\""));
}

#[test]
fn node_styles_are_rendered_without_raw_css_injection() {
    let model = parse_flowchart(
        "flowchart LR\n  A[Styled]\n  style A fill:#f00,stroke-width:2,color:#00f\n  style A fill:url(javascript:alert(1)),font-family:Arial",
    )
    .expect("styles should parse");

    let svg = render_flowchart(&model).svg;
    assert!(svg.contains("fill:#f00"));
    assert!(svg.contains("stroke-width:2"));
    assert!(svg.contains("fill:#00f"));
    assert!(svg.contains("font-family:Arial"));
    assert!(!svg.contains("javascript"));
    assert!(!svg.contains("url("));
    assert!(!svg.contains("<style"));
}
