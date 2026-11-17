use crate::errors::{PelersError, syntax_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Tb,
    Bt,
    Lr,
    Rl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Rectangle,
    Rounded,
    Diamond,
    Circle,
    Stadium,
    Cylinder,
    Hexagon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub marker: String,
    pub endpoint: EdgeEndpoint,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEndpoint {
    None,
    Circle,
    Cross,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subgraph {
    pub id: String,
    pub label: String,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowchartModel {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
}

fn direction(value: &str, source: &str, offset: usize) -> Result<Direction, PelersError> {
    match value {
        "TB" | "TD" => Ok(Direction::Tb),
        "BT" => Ok(Direction::Bt),
        "LR" => Ok(Direction::Lr),
        "RL" => Ok(Direction::Rl),
        _ => Err(syntax_error(
            "flowchart",
            source,
            offset,
            "unknown flowchart direction",
        )),
    }
}

fn add_node(nodes: &mut Vec<Node>, id: &str, label: Option<&str>, shape: Shape) {
    if let Some(node) = nodes.iter_mut().find(|node| node.id == id) {
        if let Some(label) = label {
            node.label = label.trim().to_owned();
            node.shape = shape;
        }
        return;
    }
    nodes.push(Node {
        id: id.to_owned(),
        label: label.map_or_else(|| id.to_owned(), |label| label.trim().to_owned()),
        shape,
        styles: Vec::new(),
    });
}

fn parse_node(value: &str) -> (&str, Option<&str>, Shape) {
    for (open, close, shape) in [
        ("((", "))", Shape::Circle),
        ("([", "])", Shape::Stadium),
        ("[(", ")]", Shape::Cylinder),
        ("{{", "}}", Shape::Hexagon),
        ("[", "]", Shape::Rectangle),
        ("(", ")", Shape::Rounded),
        ("{", "}", Shape::Diamond),
    ] {
        if let Some(start) = value.find(open)
            && value.ends_with(close)
        {
            return (
                &value[..start],
                Some(&value[start + open.len()..value.len() - close.len()]),
                shape,
            );
        }
    }
    (value, None, Shape::Rectangle)
}

fn parse_edge_label<'a>(
    value: &'a str,
    source: &str,
    offset: usize,
) -> Result<(Option<String>, &'a str), PelersError> {
    if let Some(rest) = value.strip_prefix('|') {
        let Some(end) = rest.find('|') else {
            return Err(syntax_error(
                "flowchart",
                source,
                offset,
                "unterminated edge label",
            ));
        };
        return Ok((Some(rest[..end].trim().to_owned()), rest[end + 1..].trim()));
    }
    Ok((None, value))
}

fn edge_marker(statement: &str) -> Option<(&'static str, EdgeEndpoint)> {
    [
        ("-.->", EdgeEndpoint::None),
        ("==>", EdgeEndpoint::None),
        ("-->", EdgeEndpoint::None),
        ("---", EdgeEndpoint::None),
        ("--o", EdgeEndpoint::Circle),
        ("--x", EdgeEndpoint::Cross),
    ]
    .into_iter()
    .find(|(marker, _)| statement.contains(marker))
}

fn split_semicolons(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut depth: usize = 0;
    let mut in_edge_label = false;
    let mut in_comment = false;

    for character in source.chars() {
        if in_comment {
            result.push(character);
            if character == '\n' {
                in_comment = false;
            }
            continue;
        }
        match character {
            '%' if result
                .rsplit('\n')
                .next()
                .is_some_and(|line| line.trim_start() == "%") =>
            {
                in_comment = true;
                result.push(character);
            }
            '[' | '(' | '{' if !in_edge_label => {
                depth += 1;
                result.push(character);
            }
            ']' | ')' | '}' if !in_edge_label => {
                depth = depth.saturating_sub(1);
                result.push(character);
            }
            '|' => {
                in_edge_label = !in_edge_label;
                result.push(character);
            }
            ';' if depth == 0 && !in_edge_label => result.push('\n'),
            _ => result.push(character),
        }
    }
    result
}

pub fn parse(source: &str) -> Result<FlowchartModel, PelersError> {
    let normalized = split_semicolons(source);
    let mut lines = normalized.lines();
    let header = lines
        .next()
        .ok_or_else(|| syntax_error("flowchart", source, 0, "missing flowchart header"))?;
    let header = header.trim();
    let (keyword, direction_value) = header
        .split_once(char::is_whitespace)
        .ok_or_else(|| syntax_error("flowchart", source, 0, "expected flowchart direction"))?;
    if keyword != "flowchart" && keyword != "graph" {
        return Err(syntax_error(
            "flowchart",
            source,
            0,
            "expected 'flowchart' or 'graph'",
        ));
    }
    let direction = direction(direction_value.trim(), source, keyword.len() + 1)?;
    let mut model = FlowchartModel {
        direction,
        nodes: Vec::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
    };

    let mut offset = header.len() + 1;
    let mut active_subgraph: Option<Subgraph> = None;
    for line in lines {
        let statement = line.trim();
        if statement.is_empty() || statement.starts_with("%%") {
            offset += line.len() + 1;
            continue;
        }
        if let Some(value) = statement.strip_prefix("subgraph") {
            if active_subgraph.is_some() {
                return Err(syntax_error(
                    "flowchart",
                    source,
                    offset,
                    "nested subgraphs are not supported",
                ));
            }
            let label = value.trim();
            if label.is_empty() {
                return Err(syntax_error(
                    "flowchart",
                    source,
                    offset,
                    "subgraph requires an identifier",
                ));
            }
            active_subgraph = Some(Subgraph {
                id: label
                    .split_once('[')
                    .map_or(label, |(id, _)| id.trim())
                    .to_owned(),
                label: label
                    .strip_prefix('[')
                    .and_then(|value| value.strip_suffix(']'))
                    .unwrap_or(label)
                    .to_owned(),
                nodes: Vec::new(),
            });
            offset += line.len() + 1;
            continue;
        }
        if statement == "end" {
            let Some(subgraph) = active_subgraph.take() else {
                return Err(syntax_error(
                    "flowchart",
                    source,
                    offset,
                    "unexpected subgraph end",
                ));
            };
            model.subgraphs.push(subgraph);
            offset += line.len() + 1;
            continue;
        }
        if let Some(style) = statement.strip_prefix("style ") {
            let mut parts = style.splitn(2, char::is_whitespace);
            let id = parts.next().unwrap_or_default().trim();
            let declarations = parts.next().unwrap_or_default().trim();
            if id.is_empty() || declarations.is_empty() || id.contains(char::is_whitespace) {
                return Err(syntax_error(
                    "flowchart",
                    source,
                    offset,
                    "style requires a node identifier and declarations",
                ));
            }
            add_node(&mut model.nodes, id, None, Shape::Rectangle);
            for node in model.nodes.iter_mut().filter(|node| node.id == id) {
                node.styles.extend(
                    declarations
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            }
            if let Some(subgraph) = &mut active_subgraph
                && !subgraph.nodes.contains(&id.to_owned())
            {
                subgraph.nodes.push(id.to_owned());
            }
            offset += line.len() + 1;
            continue;
        }
        let before = model.nodes.len();
        if let Some((marker, endpoint)) = edge_marker(statement) {
            let endpoints = statement.split(marker).collect::<Vec<_>>();
            let mut parsed = Vec::with_capacity(endpoints.len());
            for (index, endpoint) in endpoints.iter().enumerate() {
                let (edge_label, node_text) = if index == 0 {
                    (None, endpoint.trim())
                } else {
                    parse_edge_label(endpoint.trim(), source, offset)?
                };
                let (id, label, shape) = parse_node(node_text);
                if id.is_empty() || id.contains(char::is_whitespace) {
                    return Err(syntax_error(
                        "flowchart",
                        source,
                        offset,
                        "edge endpoints cannot be empty or contain whitespace",
                    ));
                }
                add_node(&mut model.nodes, id, label, shape);
                parsed.push((id.to_owned(), edge_label));
            }
            for pair in parsed.windows(2) {
                model.edges.push(Edge {
                    from: pair[0].0.clone(),
                    to: pair[1].0.clone(),
                    marker: marker.to_owned(),
                    endpoint,
                    label: pair[1].1.clone(),
                });
            }
        } else {
            let (id, label, shape) = parse_node(statement);
            if id.is_empty() || id.contains(char::is_whitespace) {
                return Err(syntax_error(
                    "flowchart",
                    source,
                    offset,
                    "invalid node statement",
                ));
            }

            add_node(&mut model.nodes, id, label, shape);
        }
        if let Some(subgraph) = &mut active_subgraph {
            subgraph
                .nodes
                .extend(model.nodes[before..].iter().map(|node| node.id.clone()));
        }
        offset += line.len() + 1;
    }
    if active_subgraph.is_some() {
        return Err(syntax_error(
            "flowchart",
            source,
            offset,
            "unterminated subgraph",
        ));
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::{Direction, Shape, parse};

    #[test]
    fn parses_nodes_edges_and_direction() {
        let model = parse("flowchart LR\n  A[Start] --> B[End]").unwrap();
        assert_eq!(model.direction, Direction::Lr);
        assert_eq!(model.nodes[0].label, "Start");
        assert_eq!(model.nodes[0].shape, Shape::Rectangle);
        assert_eq!(model.edges[0].from, "A");
        assert_eq!(model.edges[0].to, "B");
        assert_eq!(model.edges[0].label, None);
    }

    #[test]
    fn supports_graph_td_alias() {
        assert_eq!(parse("graph TD\n  A").unwrap().direction, Direction::Tb);
    }

    #[test]
    fn rejects_invalid_direction() {
        assert!(parse("flowchart XX").is_err());
    }

    #[test]
    fn rejects_empty_edge_endpoint() {
        assert!(parse("flowchart LR\n  A -->").is_err());
    }

    #[test]
    fn parses_chained_edges_and_shapes() {
        let model = parse("flowchart LR\n  A(Start) -->|yes| B{Choice} --> C").unwrap();
        assert_eq!(model.edges.len(), 2);
        assert_eq!(model.edges[0].label.as_deref(), Some("yes"));
        assert_eq!(model.nodes[0].shape, Shape::Rounded);
        assert_eq!(model.nodes[1].shape, Shape::Diamond);
    }

    #[test]
    fn parses_extended_mermaid_shapes_before_single_delimiters() {
        let model = parse(
            "flowchart LR\n  circle((Circle)) --> stadium([Stadium])\n  stadium --> cylinder[(Cylinder)]\n  cylinder --> hexagon{{Hexagon}}",
        )
        .unwrap();
        assert_eq!(
            model
                .nodes
                .iter()
                .map(|node| node.shape)
                .collect::<Vec<_>>(),
            [
                Shape::Circle,
                Shape::Stadium,
                Shape::Cylinder,
                Shape::Hexagon
            ]
        );
        assert_eq!(model.nodes[0].label, "Circle");
        assert_eq!(model.nodes[3].label, "Hexagon");
    }

    #[test]
    fn rejects_unterminated_edge_label() {
        assert!(parse("flowchart LR\n  A -->|yes B").is_err());
    }

    #[test]
    fn preserves_non_default_edge_markers() {
        let model = parse("flowchart LR\n  A -.-> B\n  B ==> C\n  C --- D").unwrap();
        assert_eq!(model.edges[0].marker, "-.->");
        assert_eq!(model.edges[1].marker, "==>");
        assert_eq!(model.edges[2].marker, "---");
        assert!(
            model
                .edges
                .iter()
                .all(|edge| { edge.endpoint == super::EdgeEndpoint::None })
        );
    }

    #[test]
    fn parses_circle_and_cross_edge_endpoints() {
        let model = parse("flowchart LR\n  A --o B\n  B --x C").unwrap();
        assert_eq!(model.edges[0].marker, "--o");
        assert_eq!(model.edges[0].endpoint, super::EdgeEndpoint::Circle);
        assert_eq!(model.edges[1].marker, "--x");
        assert_eq!(model.edges[1].endpoint, super::EdgeEndpoint::Cross);
    }

    #[test]
    fn parses_subgraph_members() {
        let model = parse("flowchart LR\n  subgraph Group\n    A --> B\n  end").unwrap();
        assert_eq!(model.subgraphs[0].label, "Group");
        assert_eq!(model.subgraphs[0].nodes, ["A", "B"]);
    }

    #[test]
    fn parses_semicolon_separated_statements() {
        let model = parse("flowchart LR\n  A --> B; B --> C").unwrap();
        assert_eq!(model.nodes.len(), 3);
        assert_eq!(model.edges.len(), 2);
    }

    #[test]
    fn preserves_semicolons_in_labels_comments_and_subgraphs() {
        let model =
            parse("flowchart LR; %% A; comment\n subgraph Group; A[one; two] -->|yes; no| B; end")
                .unwrap();
        assert_eq!(model.nodes[0].label, "one; two");
        assert_eq!(model.edges[0].label.as_deref(), Some("yes; no"));
        assert_eq!(model.subgraphs[0].nodes, ["A", "B"]);
    }

    #[test]
    fn parses_repeated_node_style_statements() {
        let model = parse(
            "flowchart LR\n  A[Styled]\n  style A fill:#abc,stroke-width:2\n  style A color:blue",
        )
        .unwrap();
        assert_eq!(
            model.nodes[0].styles,
            ["fill:#abc", "stroke-width:2", "color:blue"]
        );
    }

    #[test]
    fn rejects_malformed_style_statements() {
        assert!(parse("flowchart LR\n  style A").is_err());
        assert!(parse("flowchart LR\n  style A fill:red\n  style A").is_err());
    }

    #[test]
    fn rejects_malformed_headers() {
        assert!(parse("").is_err());
        assert!(parse("flowchart").is_err());
        assert!(parse("chart LR").is_err());
        assert_eq!(parse("flowchart BT").unwrap().direction, Direction::Bt);
    }

    #[test]
    fn relabels_a_node_declared_twice() {
        let model = parse("flowchart LR\n  A\n  A[Named]").unwrap();
        assert_eq!(model.nodes.len(), 1);
        assert_eq!(model.nodes[0].label, "Named");
    }

    #[test]
    fn groups_nodes_into_subgraphs() {
        let model = parse(
            "flowchart LR\n  subgraph one [First]\n    A --> B\n    style A fill:#f00\n    style C fill:#00f\n  end",
        )
        .unwrap();
        assert_eq!(model.subgraphs[0].id, "one");
        assert_eq!(model.subgraphs[0].nodes, ["A", "B", "C"]);
        let bracketed = parse("flowchart LR\n  subgraph [Only]\n    A\n  end").unwrap();
        assert_eq!(bracketed.subgraphs[0].label, "Only");
    }

    #[test]
    fn rejects_misplaced_subgraph_keywords() {
        assert!(parse("flowchart LR\n  subgraph a\n  subgraph b\n  end\n  end").is_err());
        assert!(parse("flowchart LR\n  subgraph\n  end").is_err());
        assert!(parse("flowchart LR\n  end").is_err());
        assert!(parse("flowchart LR\n  subgraph a\n    A").is_err());
    }
}
