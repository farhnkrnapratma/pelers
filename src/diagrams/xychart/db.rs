use crate::diagrams::common::{acc_descr, acc_title};
use crate::util::chars::trim;
use crate::util::jsnum::js_number_to_string;

#[derive(Clone, Debug, PartialEq)]
pub struct XyText {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    pub value: f64,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XAxis {
    Band {
        title: String,
        categories: Vec<String>,
    },
    Linear {
        title: String,
        min: f64,
        max: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct YAxis {
    pub title: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plot {
    pub bar: bool,
    pub title: String,
    pub data: Vec<(String, Option<f64>)>,
    pub point_labels: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XyChartDb {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub orientation: Option<&'static str>,
    pub x_axis: XAxis,
    pub y_axis: YAxis,
    pub has_set_x: bool,
    pub has_set_y: bool,
    pub plots: Vec<Plot>,
}

impl Default for XyChartDb {
    fn default() -> Self {
        Self {
            title: None,
            acc_title: None,
            acc_descr: None,
            orientation: None,
            x_axis: XAxis::Band {
                title: String::new(),
                categories: Vec::new(),
            },
            y_axis: YAxis {
                title: String::new(),
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
            },
            has_set_x: false,
            has_set_y: false,
            plots: Vec::new(),
        }
    }
}

fn x_title(axis: &XAxis) -> String {
    match axis {
        XAxis::Band { title, .. } | XAxis::Linear { title, .. } => title.clone(),
    }
}

impl XyChartDb {
    pub fn set_orientation(&mut self, orientation: &str) {
        self.orientation = Some(if orientation == "horizontal" {
            "horizontal"
        } else {
            "vertical"
        });
    }

    pub fn set_x_axis_title(&mut self, title: XyText) {
        let text = trim(&title.text).to_owned();
        match &mut self.x_axis {
            XAxis::Band { title, .. } | XAxis::Linear { title, .. } => *title = text,
        }
    }

    pub fn set_x_axis_range(&mut self, min: f64, max: f64) {
        self.x_axis = XAxis::Linear {
            title: x_title(&self.x_axis),
            min,
            max,
        };
        self.has_set_x = true;
    }

    pub fn set_x_axis_band(&mut self, categories: Vec<XyText>) {
        self.x_axis = XAxis::Band {
            title: x_title(&self.x_axis),
            categories: categories
                .iter()
                .map(|c| trim(&c.text).to_owned())
                .collect(),
        };
        self.has_set_x = true;
    }

    pub fn set_y_axis_title(&mut self, title: XyText) {
        self.y_axis.title = trim(&title.text).to_owned();
    }

    pub fn set_y_axis_range(&mut self, min: f64, max: f64) {
        self.y_axis.min = min;
        self.y_axis.max = max;
        self.has_set_y = true;
    }

    fn plot_data(&mut self, mut values: Vec<f64>) -> Vec<(String, Option<f64>)> {
        if values.is_empty() {
            return Vec::new();
        }
        if !self.has_set_x {
            self.set_x_axis_range(1.0, values.len() as f64);
        }
        if let XAxis::Band { categories, .. } = &self.x_axis
            && values.len() > categories.len()
        {
            values.truncate(categories.len());
        }
        if !self.has_set_y {
            let mut min = self.y_axis.min;
            let mut max = self.y_axis.max;
            for &value in &values {
                if value < min {
                    min = value;
                }
                if value > max {
                    max = value;
                }
            }
            self.y_axis.min = min;
            self.y_axis.max = max;
        }
        match &self.x_axis {
            XAxis::Band { categories, .. } => categories
                .iter()
                .enumerate()
                .map(|(i, category)| (category.clone(), values.get(i).copied()))
                .collect(),
            XAxis::Linear { min, max, .. } => {
                if values.len() == 1 {
                    return vec![(js_number_to_string(*min), Some(values[0]))];
                }
                let step = (max - min) / (values.len() - 1) as f64;
                values
                    .iter()
                    .enumerate()
                    .map(|(i, &value)| (js_number_to_string(min + i as f64 * step), Some(value)))
                    .collect()
            }
        }
    }

    fn add_plot(&mut self, bar: bool, title: XyText, data: Vec<DataPoint>) {
        let values: Vec<f64> = data.iter().map(|d| d.value).collect();
        let plot_data = self.plot_data(values);
        let point_labels = if !bar && data.iter().any(|d| !trim(&d.label).is_empty()) {
            Some(data.iter().map(|d| trim(&d.label).to_owned()).collect())
        } else {
            None
        };
        self.plots.push(Plot {
            bar,
            title: trim(&title.text).to_owned(),
            data: plot_data,
            point_labels,
        });
    }

    pub fn set_line_data(&mut self, title: XyText, data: Vec<DataPoint>) {
        self.add_plot(false, title, data);
    }

    pub fn set_bar_data(&mut self, title: XyText, data: Vec<DataPoint>) {
        self.add_plot(true, title, data);
    }

    pub fn set_diagram_title(&mut self, text: &str) {
        self.title = Some(text.to_owned());
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(text.to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = Some(text.to_owned());
    }

    pub fn finish(mut self) -> Self {
        self.acc_title = acc_title(self.acc_title.take());
        self.acc_descr = acc_descr(self.acc_descr.take());
        self
    }
}
