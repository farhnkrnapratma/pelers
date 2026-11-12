use super::parser::{ComponentAst, LabelAst, WardleyAst};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::{is_js_space, trim};
use crate::util::jsnum::js_number_to_string;
use crate::util::ordered::OrderedMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Forward,
    Backward,
    Bidirectional,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyNode {
    pub id: String,
    pub label: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub class_name: Option<String>,
    pub label_offset_x: Option<f64>,
    pub label_offset_y: Option<f64>,
    pub in_pipeline: bool,
    pub is_pipeline_parent: bool,
    pub inertia: bool,
    pub source_strategy: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyLink {
    pub source: String,
    pub target: String,
    pub dashed: bool,
    pub label: Option<String>,
    pub flow: Option<Flow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyTrend {
    pub node_id: String,
    pub target_x: f64,
    pub target_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyPipeline {
    pub node_id: String,
    pub component_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyAnnotation {
    pub number: f64,
    pub coordinates: Vec<(f64, f64)>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyNote {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyForce {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WardleyAxes {
    pub stages: Option<Vec<String>>,
    pub stage_boundaries: Option<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WardleyModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub nodes: Vec<WardleyNode>,
    pub links: Vec<WardleyLink>,
    pub trends: Vec<WardleyTrend>,
    pub pipelines: Vec<WardleyPipeline>,
    pub annotations: Vec<WardleyAnnotation>,
    pub notes: Vec<WardleyNote>,
    pub accelerators: Vec<WardleyForce>,
    pub deaccelerators: Vec<WardleyForce>,
    pub annotations_box: Option<(f64, f64)>,
    pub axes: WardleyAxes,
    pub size: Option<(f64, f64)>,
}

fn semantic(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("wardley".to_owned());
    error
}

struct NodeInput {
    id: String,
    label: String,
    x: f64,
    y: f64,
    class_name: Option<String>,
    label_offset_x: Option<f64>,
    label_offset_y: Option<f64>,
    inertia: bool,
    source_strategy: Option<String>,
}

#[derive(Default)]
struct Builder {
    nodes: OrderedMap<WardleyNode>,
    links: Vec<WardleyLink>,
    trends: OrderedMap<WardleyTrend>,
    pipelines: OrderedMap<WardleyPipeline>,
    annotations: Vec<WardleyAnnotation>,
    notes: Vec<WardleyNote>,
    accelerators: Vec<WardleyForce>,
    deaccelerators: Vec<WardleyForce>,
    annotations_box: Option<(f64, f64)>,
    axes: WardleyAxes,
    size: Option<(f64, f64)>,
}

impl Builder {
    fn add_node(&mut self, node: NodeInput) {
        let existing = self.nodes.get(&node.id).cloned();
        let (in_pipeline, is_pipeline_parent) = existing
            .as_ref()
            .map_or((false, false), |e| (e.in_pipeline, e.is_pipeline_parent));
        let class_name = node
            .class_name
            .or_else(|| existing.as_ref().and_then(|e| e.class_name.clone()));
        let label_offset_x = node
            .label_offset_x
            .or_else(|| existing.as_ref().and_then(|e| e.label_offset_x));
        let label_offset_y = node
            .label_offset_y
            .or_else(|| existing.as_ref().and_then(|e| e.label_offset_y));
        let merged = WardleyNode {
            id: node.id.clone(),
            label: node.label,
            x: Some(node.x),
            y: Some(node.y),
            class_name,
            label_offset_x,
            label_offset_y,
            in_pipeline,
            is_pipeline_parent,
            inertia: node.inertia,
            source_strategy: node.source_strategy,
        };
        self.nodes.set(&node.id, merged);
    }

    fn start_pipeline(&mut self, node_id: &str) {
        self.pipelines.set(
            node_id,
            WardleyPipeline {
                node_id: node_id.to_owned(),
                component_ids: Vec::new(),
            },
        );
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.is_pipeline_parent = true;
        }
    }

    fn add_pipeline_component(&mut self, parent: &str, component: &str) {
        if let Some(pipeline) = self.pipelines.get_mut(parent) {
            pipeline.component_ids.push(component.to_owned());
        }
        if let Some(node) = self.nodes.get_mut(component) {
            node.in_pipeline = true;
        }
    }

    fn resolve_node_id(&self, name: &str) -> String {
        if self.nodes.contains(name) {
            return name.to_owned();
        }
        for (id, node) in self.nodes.iter() {
            if node.label == name {
                return id.to_owned();
            }
        }
        name.to_owned()
    }

    fn build(&self) -> Result<(Vec<WardleyNode>, ()), PelersError> {
        let mut nodes = Vec::new();
        for (_, node) in self.nodes.iter() {
            if node.x.is_none() || node.y.is_none() {
                return Err(semantic(format!(
                    "Node \"{}\" is missing coordinates",
                    node.label
                )));
            }
            nodes.push(node.clone());
        }
        Ok((nodes, ()))
    }
}

fn to_percent(value: f64, context: &str) -> Result<f64, PelersError> {
    let normalized = if value <= 1.0 { value * 100.0 } else { value };
    if normalized < 0.0 || normalized > 100.0 {
        return Err(semantic(format!(
            "{context} must be between 0-1 (decimal) or 0-100 (percentage). Received: {}",
            js_number_to_string(value)
        )));
    }
    Ok(normalized)
}

fn to_coordinates(
    visibility: f64,
    evolution: f64,
    context: &str,
) -> Result<(f64, f64), PelersError> {
    let x = to_percent(evolution, &format!("{context} evolution"))?;
    let y = to_percent(visibility, &format!("{context} visibility"))?;
    Ok((x, y))
}

fn flow_from_port(port: Option<&str>) -> Option<Flow> {
    match port {
        Some("+<>") => Some(Flow::Bidirectional),
        Some("+<") => Some(Flow::Backward),
        Some("+>") => Some(Flow::Forward),
        _ => None,
    }
}

struct ArrowFlow {
    flow: Option<Flow>,
    label: Option<String>,
}

fn flow_from_arrow(arrow: Option<&str>) -> ArrowFlow {
    let Some(arrow) = arrow.filter(|a| a.starts_with('+')) else {
        return ArrowFlow {
            flow: None,
            label: None,
        };
    };
    let end = arrow
        .get(2..)
        .and_then(|rest| rest.find('\''))
        .map(|i| i + 2);
    let label = match (arrow.as_bytes().get(1), end) {
        (Some(b'\''), Some(end)) => Some(arrow[2..end].to_owned()),
        _ => None,
    };
    if arrow.contains("<>") {
        return ArrowFlow {
            flow: Some(Flow::Bidirectional),
            label,
        };
    }
    if arrow.contains('<') {
        return ArrowFlow {
            flow: Some(Flow::Backward),
            label,
        };
    }
    if arrow.contains('>') {
        return ArrowFlow {
            flow: Some(Flow::Forward),
            label,
        };
    }
    ArrowFlow { flow: None, label }
}

fn offsets(label: Option<&LabelAst>) -> (Option<f64>, Option<f64>) {
    match label {
        Some(label) => (
            Some(if label.neg_x { -1.0 } else { 1.0 } * label.offset_x),
            Some(if label.neg_y { -1.0 } else { 1.0 } * label.offset_y),
        ),
        None => (None, None),
    }
}

fn populate(ast: &WardleyAst, db: &mut Builder) -> Result<(), PelersError> {
    if let Some(size) = ast.size {
        db.size = Some(size);
    }

    if let Some(stages) = &ast.evolution {
        let names: Vec<String> = stages
            .iter()
            .map(|stage| match &stage.second_name {
                Some(second) => format!("{} / {}", trim(&stage.name), trim(second)),
                None => trim(&stage.name).to_owned(),
            })
            .collect();
        let boundaries: Vec<f64> = stages.iter().filter_map(|stage| stage.boundary).collect();
        db.axes = WardleyAxes {
            stages: Some(names),
            stage_boundaries: Some(boundaries),
        };
    }

    for anchor in &ast.anchors {
        let (x, y) = to_coordinates(
            anchor.visibility,
            anchor.evolution,
            &format!("Anchor \"{}\"", anchor.name),
        )?;
        db.add_node(NodeInput {
            id: anchor.name.clone(),
            label: anchor.name.clone(),
            x,
            y,
            class_name: Some("anchor".to_owned()),
            label_offset_x: None,
            label_offset_y: None,
            inertia: false,
            source_strategy: None,
        });
    }

    for component in &ast.components {
        add_component(db, component)?;
    }

    for note in &ast.notes {
        let (x, y) = to_coordinates(
            note.visibility,
            note.evolution,
            &format!("Note \"{}\"", note.text),
        )?;
        db.notes.push(WardleyNote {
            text: note.text.clone(),
            x,
            y,
        });
    }

    for (parent_name, components) in &ast.pipelines {
        let parent_y = match db.nodes.get(parent_name) {
            Some(parent) => match parent.y {
                Some(y) => y,
                None => {
                    return Err(semantic(format!(
                        "Pipeline \"{parent_name}\" must reference an existing component with coordinates."
                    )));
                }
            },
            None => {
                return Err(semantic(format!(
                    "Pipeline \"{parent_name}\" must reference an existing component with coordinates."
                )));
            }
        };
        db.start_pipeline(parent_name);
        for component in components {
            let id = format!("{parent_name}_{}", component.name);
            let (label_offset_x, label_offset_y) = offsets(component.label.as_ref());
            let x = to_percent(
                component.evolution,
                &format!("Pipeline component \"{}\" evolution", component.name),
            )?;
            db.add_node(NodeInput {
                id: id.clone(),
                label: component.name.clone(),
                x,
                y: parent_y,
                class_name: Some("pipeline-component".to_owned()),
                label_offset_x,
                label_offset_y,
                inertia: false,
                source_strategy: None,
            });
            db.add_pipeline_component(parent_name, &id);
        }
    }

    for link in &ast.links {
        let dashed = link
            .arrow
            .as_deref()
            .is_some_and(|a| a.contains("-.->") || a.contains(".-."));
        let from_arrow = flow_from_arrow(link.arrow.as_deref());
        let flow = flow_from_port(link.from_port.as_deref())
            .or_else(|| flow_from_port(link.to_port.as_deref()))
            .or(from_arrow.flow);
        db.links.push(WardleyLink {
            source: db.resolve_node_id(&link.from),
            target: db.resolve_node_id(&link.to),
            dashed,
            label: from_arrow.label.or_else(|| link.link_label.clone()),
            flow,
        });
    }

    for (component, target) in &ast.evolves {
        let node_y = db.nodes.get(component).and_then(|node| node.y);
        if let Some(y) = node_y {
            let target_x = to_percent(*target, &format!("Evolve target for \"{component}\""))?;
            db.trends.set(
                component,
                WardleyTrend {
                    node_id: component.clone(),
                    target_x,
                    target_y: y,
                },
            );
        }
    }

    if let Some(&(x, y)) = ast.annotations.first() {
        let (bx, by) = to_coordinates(x, y, "Annotations box")?;
        db.annotations_box = Some((bx, by));
    }

    for annotation in &ast.annotation {
        let (x, y) = to_coordinates(
            annotation.x,
            annotation.y,
            &format!("Annotation {}", js_number_to_string(annotation.number)),
        )?;
        db.annotations.push(WardleyAnnotation {
            number: annotation.number,
            coordinates: vec![(x, y)],
            text: annotation.text.clone(),
        });
    }

    for accelerator in &ast.accelerators {
        let (x, y) = to_coordinates(
            accelerator.x,
            accelerator.y,
            &format!("Accelerator \"{}\"", accelerator.name),
        )?;
        db.accelerators.push(WardleyForce {
            name: accelerator.name.clone(),
            x,
            y,
        });
    }

    for deaccelerator in &ast.deaccelerators {
        let (x, y) = to_coordinates(
            deaccelerator.x,
            deaccelerator.y,
            &format!("Deaccelerator \"{}\"", deaccelerator.name),
        )?;
        db.deaccelerators.push(WardleyForce {
            name: deaccelerator.name.clone(),
            x,
            y,
        });
    }

    Ok(())
}

fn add_component(db: &mut Builder, component: &ComponentAst) -> Result<(), PelersError> {
    let (x, y) = to_coordinates(
        component.visibility,
        component.evolution,
        &format!("Component \"{}\"", component.name),
    )?;
    let (label_offset_x, label_offset_y) = offsets(component.label.as_ref());
    db.add_node(NodeInput {
        id: component.name.clone(),
        label: component.name.clone(),
        x,
        y,
        class_name: Some("component".to_owned()),
        label_offset_x,
        label_offset_y,
        inertia: component.inertia,
        source_strategy: component.decorator.as_ref().map(|d| d.strategy.clone()),
    });
    Ok(())
}

pub fn build_wardley(ast: &WardleyAst, title: Option<&str>) -> Result<WardleyModel, PelersError> {
    let mut db = Builder::default();
    populate(ast, &mut db)?;
    let (nodes, ()) = db.build()?;
    let title = match &ast.title {
        Some(title) if !title.is_empty() => Some(title.clone()),
        _ => title.map(str::to_owned),
    };
    Ok(WardleyModel {
        title,
        acc_title: ast
            .acc_title
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| t.trim_start_matches(is_js_space).to_owned()),
        acc_descr: ast
            .acc_descr
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(collapse_indented_lines),
        nodes,
        links: db.links,
        trends: db.trends.values().cloned().collect(),
        pipelines: db.pipelines.values().cloned().collect(),
        annotations: db.annotations,
        notes: db.notes,
        accelerators: db.accelerators,
        deaccelerators: db.deaccelerators,
        annotations_box: db.annotations_box,
        axes: db.axes,
        size: db.size,
    })
}

fn collapse_indented_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' && chars.peek().is_some_and(|&n| is_js_space(n)) {
            out.push('\n');
            while chars.peek().is_some_and(|&n| is_js_space(n)) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}
