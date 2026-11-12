#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderOptions {
    pub icons: Option<crate::util::hook::Hook<crate::text::icons::IconFn>>,
    pub font_family: Option<String>,
    pub font_family_mono: Option<String>,
    pub font_size: Option<f64>,
    pub id_prefix: Option<String>,
    pub responsive: Option<bool>,
    pub max_width: Option<f64>,
    pub auto_direction: Option<bool>,
    pub direction_breakpoint: Option<f64>,
    pub padding: Option<f64>,
    pub max_edges: Option<usize>,
    pub link_schemes: Option<Vec<String>>,
    pub image_schemes: Option<Vec<String>>,
    pub links: Option<bool>,
    pub images: Option<bool>,
    pub link_rel: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkInfo {
    pub id: String,
    pub href: String,
    pub internal: bool,
}

pub fn link_options(options: &crate::svg::root::RenderOptions) -> RenderOptions {
    RenderOptions {
        link_schemes: options.link_schemes.clone(),
        image_schemes: options.image_schemes.clone(),
        links: options.links,
        images: options.images,
        link_rel: options.link_rel.clone(),
        ..RenderOptions::default()
    }
}
