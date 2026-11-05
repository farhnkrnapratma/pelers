use std::io::{self, Read};

use pelers::{detect_diagram, parse_flowchart, preprocess, render_flowchart};

fn main() -> io::Result<()> {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;

    let prepared = preprocess(&source)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;

    match detect_diagram(&prepared.text) {
        Some(found) => {
            println!("{:?} ({:?})", found.diagram_type, found.mermaid_type);
            if matches!(found.diagram_type, pelers::DiagramType::Flowchart) {
                let model = parse_flowchart(&prepared.text).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
                })?;
                println!("nodes={} edges={}", model.nodes.len(), model.edges.len());
                println!("{}", render_flowchart(&model).svg);
            }
        }
        None => println!("No diagram type detected"),
    }

    Ok(())
}
