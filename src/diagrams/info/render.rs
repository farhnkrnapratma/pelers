use super::InfoModel;
use crate::diagrams::common::Drawn;
use crate::svg::builder::label_svg;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::text::label::layout_label;
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::js_max;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render_info(model: &InfoModel, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let pad = options.padding.unwrap_or(8.0);

    let title = layout_label(model.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 8.0
    } else {
        0.0
    };
    let text = layout_label(Some(&format!("Pele v{VERSION}")), false, size, 4000.0, 0);

    let width = (js_max(title.width, text.width) + 2.0 * pad).ceil();
    let height = (title_height + text.height + 2.0 * pad).ceil();
    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &label_svg(
        &text,
        width / 2.0,
        pad + title_height + text.height / 2.0,
        " class=\"pele-version\" fill=\"var(--_m)\"",
    );

    let acc = Accessible {
        acc_title: model.acc_title.as_deref(),
        acc_descr: model.acc_descr.as_deref(),
    };
    Drawn {
        svg: svg_document("info", width, height, size, options, acc, &content),
        width,
        height,
    }
}
