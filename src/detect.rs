#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    Flowchart,
    Sequence,
    Class,
    State,
    Er,
    Pie,
    Gantt,
    GitGraph,
    Mindmap,
    Timeline,
    Journey,
    Xychart,
    QuadrantChart,
    Requirement,
    C4,
    Sankey,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    Treemap,
    Info,
    Agentflow,
    Usecase,
    Railroad,
    TreeView,
    Venn,
    Cynefin,
    Wardley,
    Swimlane,
    Ishikawa,
    Eventmodeling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MermaidType {
    FlowchartElk,
    Mindmap,
    Architecture,
    Agentflow,
    C4,
    Kanban,
    ClassDiagram,
    Er,
    Gantt,
    Info,
    Pie,
    Requirement,
    Sequence,
    Swimlane,
    FlowchartV2,
    Timeline,
    GitGraph,
    StateDiagram,
    Journey,
    QuadrantChart,
    Sankey,
    Packet,
    Xychart,
    Block,
    Eventmodeling,
    TreeView,
    Radar,
    Ishikawa,
    Treemap,
    Railroad,
    RailroadEbnf,
    RailroadAbnf,
    RailroadPeg,
    Venn,
    Wardley,
    Cynefin,
    Usecase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    pub diagram_type: DiagramType,
    pub mermaid_type: MermaidType,
}

fn starts_with_word(text: &str, word: &str) -> bool {
    text.strip_prefix(word)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

pub fn detect_diagram(source: &str) -> Option<Detected> {
    let text = source.trim_start();
    let detected = |diagram_type, mermaid_type| {
        Some(Detected {
            diagram_type,
            mermaid_type,
        })
    };

    if text.starts_with("flowchart-elk") {
        return detected(DiagramType::Flowchart, MermaidType::FlowchartElk);
    }
    if starts_with_word(text, "mindmap") {
        return detected(DiagramType::Mindmap, MermaidType::Mindmap);
    }
    if starts_with_word(text, "architecture") {
        return detected(DiagramType::Architecture, MermaidType::Architecture);
    }
    if starts_with_word(text, "agentflow-beta") {
        return detected(DiagramType::Agentflow, MermaidType::Agentflow);
    }
    if [
        "C4Context",
        "C4Container",
        "C4Component",
        "C4Dynamic",
        "C4Deployment",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
    {
        return detected(DiagramType::C4, MermaidType::C4);
    }
    if starts_with_word(text, "kanban") {
        return detected(DiagramType::Kanban, MermaidType::Kanban);
    }
    if starts_with_word(text, "classDiagram") {
        return detected(DiagramType::Class, MermaidType::ClassDiagram);
    }
    if starts_with_word(text, "erDiagram") {
        return detected(DiagramType::Er, MermaidType::Er);
    }
    if starts_with_word(text, "gantt") {
        return detected(DiagramType::Gantt, MermaidType::Gantt);
    }
    if starts_with_word(text, "info") {
        return detected(DiagramType::Info, MermaidType::Info);
    }
    if starts_with_word(text, "pie") {
        return detected(DiagramType::Pie, MermaidType::Pie);
    }
    if text.starts_with("requirementDiagram") || starts_with_word(text, "requirement") {
        return detected(DiagramType::Requirement, MermaidType::Requirement);
    }
    if starts_with_word(text, "sequenceDiagram") {
        return detected(DiagramType::Sequence, MermaidType::Sequence);
    }
    if starts_with_word(text, "swimlane-beta") {
        return detected(DiagramType::Swimlane, MermaidType::Swimlane);
    }
    if text.starts_with("graph") || text.starts_with("flowchart") {
        return detected(DiagramType::Flowchart, MermaidType::FlowchartV2);
    }
    if starts_with_word(text, "timeline") {
        return detected(DiagramType::Timeline, MermaidType::Timeline);
    }
    if starts_with_word(text, "gitGraph") {
        return detected(DiagramType::GitGraph, MermaidType::GitGraph);
    }
    if starts_with_word(text, "stateDiagram") {
        return detected(DiagramType::State, MermaidType::StateDiagram);
    }
    if starts_with_word(text, "journey") {
        return detected(DiagramType::Journey, MermaidType::Journey);
    }
    if starts_with_word(text, "quadrantChart") {
        return detected(DiagramType::QuadrantChart, MermaidType::QuadrantChart);
    }
    if text.starts_with("sankey") {
        return detected(DiagramType::Sankey, MermaidType::Sankey);
    }
    if text.starts_with("packet") {
        return detected(DiagramType::Packet, MermaidType::Packet);
    }
    if text.starts_with("xychart") {
        return detected(DiagramType::Xychart, MermaidType::Xychart);
    }
    if text.starts_with("block") {
        return detected(DiagramType::Block, MermaidType::Block);
    }
    if starts_with_word(text, "eventmodeling") {
        return detected(DiagramType::Eventmodeling, MermaidType::Eventmodeling);
    }
    if starts_with_word(text, "treeView-beta") {
        return detected(DiagramType::TreeView, MermaidType::TreeView);
    }
    if text.starts_with("radar-beta") {
        return detected(DiagramType::Radar, MermaidType::Radar);
    }
    if text.to_ascii_lowercase().starts_with("ishikawa") {
        return detected(DiagramType::Ishikawa, MermaidType::Ishikawa);
    }
    if starts_with_word(text, "treemap") {
        return detected(DiagramType::Treemap, MermaidType::Treemap);
    }
    if text.to_ascii_lowercase().starts_with("railroad-beta") {
        return detected(DiagramType::Railroad, MermaidType::Railroad);
    }
    if text.to_ascii_lowercase().starts_with("railroad-ebnf-beta") {
        return detected(DiagramType::Railroad, MermaidType::RailroadEbnf);
    }
    if text.to_ascii_lowercase().starts_with("railroad-abnf-beta") {
        return detected(DiagramType::Railroad, MermaidType::RailroadAbnf);
    }
    if text.to_ascii_lowercase().starts_with("railroad-peg-beta") {
        return detected(DiagramType::Railroad, MermaidType::RailroadPeg);
    }
    if starts_with_word(text, "venn-beta") {
        return detected(DiagramType::Venn, MermaidType::Venn);
    }
    if text.to_ascii_lowercase().starts_with("wardley-beta") {
        return detected(DiagramType::Wardley, MermaidType::Wardley);
    }
    if text.to_ascii_lowercase().starts_with("cynefin-beta") {
        return detected(DiagramType::Cynefin, MermaidType::Cynefin);
    }
    if starts_with_word(text, "usecase-beta") {
        return detected(DiagramType::Usecase, MermaidType::Usecase);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{DiagramType, MermaidType, detect_diagram};

    #[test]
    fn trims_leading_whitespace() {
        assert_eq!(
            detect_diagram("  \n flowchart LR"),
            Some(super::Detected {
                diagram_type: DiagramType::Flowchart,
                mermaid_type: MermaidType::FlowchartV2,
            })
        );
    }

    #[test]
    fn preserves_detector_precedence() {
        assert_eq!(
            detect_diagram("flowchart-elk LR"),
            Some(super::Detected {
                diagram_type: DiagramType::Flowchart,
                mermaid_type: MermaidType::FlowchartElk,
            })
        );
    }

    #[test]
    fn detects_case_insensitive_beta_types() {
        assert_eq!(
            detect_diagram("ISHIKAWA-beta"),
            Some(super::Detected {
                diagram_type: DiagramType::Ishikawa,
                mermaid_type: MermaidType::Ishikawa,
            })
        );
    }

    #[test]
    fn returns_none_for_unknown_input() {
        assert_eq!(detect_diagram("not a diagram"), None);
    }
}
