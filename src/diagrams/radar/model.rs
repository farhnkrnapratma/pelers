use super::parser::{OptionValue, RadarAst};
use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::{PelersError, PelersErrorCode};

const MAX_TICKS: f64 = 32.0;

#[derive(Clone, Debug, PartialEq)]
pub struct RadarAxis {
    pub name: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarCurve {
    pub name: String,
    pub label: String,
    pub entries: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarOptions {
    pub show_legend: bool,
    pub ticks: f64,
    pub max: Option<f64>,
    pub min: f64,
    pub graticule: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub axes: Vec<RadarAxis>,
    pub curves: Vec<RadarCurve>,
    pub options: RadarOptions,
}

fn invalid(message: impl Into<String>) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("radar".to_owned());
    error
}

pub fn build_radar(ast: RadarAst, title: Option<String>) -> Result<RadarModel, PelersError> {
    let axes: Vec<RadarAxis> = ast
        .axes
        .iter()
        .map(|axis| RadarAxis {
            name: axis.name.clone(),
            label: axis.label.clone().unwrap_or_else(|| axis.name.clone()),
        })
        .collect();

    let mut curves = Vec::with_capacity(ast.curves.len());
    for curve in &ast.curves {
        let entries: Vec<f64> = if curve.entries[0].axis.is_none() {
            curve.entries.iter().map(|entry| entry.value).collect()
        } else {
            if axes.is_empty() {
                return Err(invalid(
                    "Axes must be populated before curves for reference entries",
                ));
            }
            let mut by_axis: Vec<(&str, f64)> = Vec::new();
            for entry in &curve.entries {
                if let Some(axis) = &entry.axis
                    && !by_axis.iter().any(|(name, _)| *name == axis.as_str())
                {
                    by_axis.push((axis.as_str(), entry.value));
                }
            }
            let mut values = Vec::with_capacity(axes.len());
            for axis in &axes {
                match by_axis.iter().find(|(name, _)| *name == axis.name) {
                    Some((_, value)) => values.push(*value),
                    None => {
                        return Err(invalid(format!("Missing entry for axis {}", axis.label)));
                    }
                }
            }
            values
        };
        curves.push(RadarCurve {
            name: curve.name.clone(),
            label: curve.label.clone().unwrap_or_else(|| curve.name.clone()),
            entries,
        });
    }

    let mut options = RadarOptions {
        show_legend: true,
        ticks: 5.0,
        max: None,
        min: 0.0,
        graticule: "circle".to_owned(),
    };
    for option in &ast.options {
        match (option.name.as_str(), &option.value) {
            ("showLegend", OptionValue::Bool(flag)) => options.show_legend = *flag,
            ("graticule", OptionValue::Text(text)) => options.graticule = text.clone(),
            ("ticks", OptionValue::Number(n)) => options.ticks = *n,
            ("max", OptionValue::Number(n)) => options.max = Some(*n),
            ("min", OptionValue::Number(n)) => options.min = *n,
            _ => {}
        }
    }
    if options.ticks > MAX_TICKS {
        options.ticks = MAX_TICKS;
    }

    Ok(RadarModel {
        title: ast.title.filter(|t| !t.is_empty()).or(title),
        acc_title: acc_title(ast.acc_title),
        acc_descr: acc_descr(ast.acc_descr),
        axes,
        curves,
        options,
    })
}
