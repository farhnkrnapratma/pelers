use crate::util::chars::{is_word, trim_start};

macro_rules! names {
    ($(#[$meta:meta])* $kind:ident { $($variant:ident => $name:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $kind {
            $($variant),+
        }

        impl $kind {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }
        }
    };
}

names! {
    DiagramType {
        Flowchart => "flowchart",
        Sequence => "sequence",
        Class => "class",
        State => "state",
        Er => "er",
        Pie => "pie",
        Gantt => "gantt",
        GitGraph => "gitGraph",
        Mindmap => "mindmap",
        Timeline => "timeline",
        Journey => "journey",
        Xychart => "xychart",
        QuadrantChart => "quadrantChart",
        Requirement => "requirement",
        C4 => "c4",
        Sankey => "sankey",
        Block => "block",
        Packet => "packet",
        Kanban => "kanban",
        Architecture => "architecture",
        Radar => "radar",
        Treemap => "treemap",
        Info => "info",
        Agentflow => "agentflow",
        Usecase => "usecase",
        Railroad => "railroad",
        TreeView => "treeView",
        Venn => "venn",
        Cynefin => "cynefin",
        Wardley => "wardley",
        Swimlane => "swimlane",
        Ishikawa => "ishikawa",
        Eventmodeling => "eventmodeling",
    }
}

names! {
    MermaidType {
        FlowchartElk => "flowchart-elk",
        Mindmap => "mindmap",
        Architecture => "architecture",
        Agentflow => "agentflow",
        C4 => "c4",
        Kanban => "kanban",
        ClassDiagram => "classDiagram",
        Er => "er",
        Gantt => "gantt",
        Info => "info",
        Pie => "pie",
        Requirement => "requirement",
        Sequence => "sequence",
        Swimlane => "swimlane",
        FlowchartV2 => "flowchart-v2",
        Timeline => "timeline",
        GitGraph => "gitGraph",
        StateDiagram => "stateDiagram",
        Journey => "journey",
        QuadrantChart => "quadrantChart",
        Sankey => "sankey",
        Packet => "packet",
        Xychart => "xychart",
        Block => "block",
        Eventmodeling => "eventmodeling",
        TreeView => "treeView",
        Radar => "radar",
        Ishikawa => "ishikawa",
        Treemap => "treemap",
        Railroad => "railroad",
        RailroadEbnf => "railroadEbnf",
        RailroadAbnf => "railroadAbnf",
        RailroadPeg => "railroadPeg",
        Venn => "venn",
        Wardley => "wardley",
        Cynefin => "cynefin",
        Usecase => "usecase",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    pub diagram_type: DiagramType,
    pub mermaid_type: MermaidType,
}

#[derive(Clone, Copy)]
enum Pattern {
    Prefix(&'static str),
    Any(&'static [&'static str]),
    Word(&'static str),
    Fold(&'static str),
    Ishikawa,
    Delimited(&'static str, bool),
}

use DiagramType as D;
use MermaidType as M;
use Pattern::{Any, Delimited, Fold, Ishikawa, Prefix, Word};

const DETECTORS: [(DiagramType, MermaidType, Pattern); 37] = [
    (D::Flowchart, M::FlowchartElk, Prefix("flowchart-elk")),
    (D::Mindmap, M::Mindmap, Prefix("mindmap")),
    (D::Architecture, M::Architecture, Prefix("architecture")),
    (D::Agentflow, M::Agentflow, Word("agentflow-beta")),
    (
        D::C4,
        M::C4,
        Any(&[
            "C4Context",
            "C4Container",
            "C4Component",
            "C4Dynamic",
            "C4Deployment",
        ]),
    ),
    (D::Kanban, M::Kanban, Prefix("kanban")),
    (D::Class, M::ClassDiagram, Prefix("classDiagram")),
    (D::Er, M::Er, Prefix("erDiagram")),
    (D::Gantt, M::Gantt, Prefix("gantt")),
    (D::Info, M::Info, Prefix("info")),
    (D::Pie, M::Pie, Prefix("pie")),
    (D::Requirement, M::Requirement, Prefix("requirement")),
    (D::Sequence, M::Sequence, Prefix("sequenceDiagram")),
    (D::Swimlane, M::Swimlane, Word("swimlane-beta")),
    (D::Flowchart, M::FlowchartV2, Any(&["graph", "flowchart"])),
    (D::Timeline, M::Timeline, Prefix("timeline")),
    (D::GitGraph, M::GitGraph, Prefix("gitGraph")),
    (D::State, M::StateDiagram, Prefix("stateDiagram")),
    (D::Journey, M::Journey, Prefix("journey")),
    (D::QuadrantChart, M::QuadrantChart, Prefix("quadrantChart")),
    (D::Sankey, M::Sankey, Prefix("sankey")),
    (D::Packet, M::Packet, Prefix("packet")),
    (D::Xychart, M::Xychart, Prefix("xychart")),
    (D::Block, M::Block, Prefix("block")),
    (D::Eventmodeling, M::Eventmodeling, Prefix("eventmodeling")),
    (D::TreeView, M::TreeView, Prefix("treeView-beta")),
    (D::Radar, M::Radar, Prefix("radar-beta")),
    (D::Ishikawa, M::Ishikawa, Ishikawa),
    (D::Treemap, M::Treemap, Prefix("treemap")),
    (D::Railroad, M::Railroad, Fold("railroad-beta")),
    (D::Railroad, M::RailroadEbnf, Fold("railroad-ebnf-beta")),
    (D::Railroad, M::RailroadAbnf, Fold("railroad-abnf-beta")),
    (D::Railroad, M::RailroadPeg, Fold("railroad-peg-beta")),
    (D::Venn, M::Venn, Prefix("venn-beta")),
    (D::Wardley, M::Wardley, Fold("wardley-beta")),
    (D::Cynefin, M::Cynefin, Delimited("cynefin-beta", true)),
    (D::Usecase, M::Usecase, Delimited("usecase-beta", false)),
];

fn strip_fold<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let head = text.get(..word.len())?;
    head.eq_ignore_ascii_case(word).then(|| &text[word.len()..])
}

fn boundary(rest: &str) -> bool {
    !rest.chars().next().is_some_and(is_word)
}

fn matches(pattern: Pattern, text: &str) -> bool {
    match pattern {
        Prefix(word) => text.starts_with(word),
        Any(words) => words.iter().any(|word| text.starts_with(word)),
        Word(word) => text.strip_prefix(word).is_some_and(boundary),
        Fold(word) => strip_fold(text, word).is_some(),
        Ishikawa => strip_fold(text, "ishikawa")
            .is_some_and(|rest| strip_fold(rest, "-beta").is_some_and(boundary) || boundary(rest)),
        Delimited(word, colon) => text.strip_prefix(word).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|c| crate::util::chars::is_js_space(c) || (colon && c == ':'))
        }),
    }
}

pub fn detect_diagram(cleaned: &str) -> Option<Detected> {
    let text = trim_start(cleaned);
    DETECTORS
        .iter()
        .find(|(_, _, pattern)| matches(*pattern, text))
        .map(|&(diagram_type, mermaid_type, _)| Detected {
            diagram_type,
            mermaid_type,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(text: &str) -> Option<(&'static str, &'static str)> {
        detect_diagram(text).map(|d| (d.diagram_type.as_str(), d.mermaid_type.as_str()))
    }

    #[test]
    fn follows_the_registration_order() {
        assert_eq!(
            kind("flowchart-elk TD"),
            Some(("flowchart", "flowchart-elk"))
        );
        assert_eq!(kind("graph TD"), Some(("flowchart", "flowchart-v2")));
        assert_eq!(kind("  \n info showInfo"), Some(("info", "info")));
        assert_eq!(kind("infographic"), Some(("info", "info")));
        assert_eq!(
            kind("requirementDiagram"),
            Some(("requirement", "requirement"))
        );
        assert_eq!(kind("sequenceDiagram\n"), Some(("sequence", "sequence")));
    }

    #[test]
    fn applies_word_boundaries_and_case_folding_where_the_oracle_does() {
        assert_eq!(kind("agentflow-beta"), Some(("agentflow", "agentflow")));
        assert_eq!(kind("agentflow-betax"), None);
        assert_eq!(kind("ISHIKAWA-Beta\n"), Some(("ishikawa", "ishikawa")));
        assert_eq!(kind("ishikawa-betax"), Some(("ishikawa", "ishikawa")));
        assert_eq!(kind("ishikawax"), None);
        assert_eq!(
            kind("RAILROAD-EBNF-BETA"),
            Some(("railroad", "railroadEbnf"))
        );
        assert_eq!(kind("railroad-beta"), Some(("railroad", "railroad")));
        assert_eq!(kind("Wardley-Beta"), Some(("wardley", "wardley")));
        assert_eq!(kind("cynefin-beta:"), Some(("cynefin", "cynefin")));
        assert_eq!(kind("cynefin-betax"), None);
        assert_eq!(kind("usecase-beta:"), None);
        assert_eq!(kind("usecase-beta"), Some(("usecase", "usecase")));
    }

    #[test]
    fn returns_none_for_unknown_input() {
        assert_eq!(kind("hello"), None);
        assert_eq!(kind(""), None);
    }
}
