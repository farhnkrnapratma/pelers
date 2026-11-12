use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::{PelersError, PelersErrorCode};

#[derive(Clone, Debug, PartialEq)]
pub struct TimelinePeriod {
    pub text: String,
    pub events: Vec<String>,
    pub section: String,
    pub section_index: isize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub vertical: bool,
    pub sections: Vec<String>,
    pub periods: Vec<TimelinePeriod>,
    current_section: String,
}

impl TimelineModel {
    pub fn add_section(&mut self, text: &str) {
        self.current_section = text.to_owned();
        self.sections.push(text.to_owned());
    }

    pub fn add_task(&mut self, period: &str) {
        self.periods.push(TimelinePeriod {
            text: period.to_owned(),
            events: Vec::new(),
            section: self.current_section.clone(),
            section_index: self.sections.len() as isize - 1,
        });
    }

    pub fn add_event(&mut self, event: &str) -> Result<(), PelersError> {
        let Some(period) = self.periods.last_mut() else {
            let mut error = PelersError::new(
                "An event needs a time period before it.",
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("timeline".to_owned());
            return Err(error);
        };
        period.events.push(event.to_owned());
        Ok(())
    }

    pub fn finish(mut self) -> Self {
        self.acc_title = acc_title(self.acc_title.take());
        self.acc_descr = acc_descr(self.acc_descr.take());
        self
    }
}
