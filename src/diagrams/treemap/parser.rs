use super::lexer::{
    ACC_DESCR_KIND, ACC_TITLE_KIND, CLASS_DEF, COMMA, ID, INDENTATION, KEYWORD, NUMBER, SEPARATOR,
    STRING, STYLE_SEPARATOR, TITLE, TREEMAP_TOKENS, class_def_parts,
};
use crate::diagrams::common::tokens::{
    Reader, acc_descr_value, acc_title_value, title_value, tokenize,
};
use crate::errors::PelersError;
use crate::util::jsnum::js_parse_float;

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Section {
        name: String,
        class_selector: Option<String>,
    },
    Leaf {
        name: String,
        value: f64,
        class_selector: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Node {
        indent: Option<usize>,
        item: Item,
    },
    ClassDef {
        indent: Option<usize>,
        class_name: String,
        style_text: Option<String>,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreemapAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub rows: Vec<Row>,
}

pub fn parse_treemap(src: &str) -> Result<TreemapAst, PelersError> {
    let mut r = Reader::new(
        tokenize(src, &TREEMAP_TOKENS, "treemap")?,
        &TREEMAP_TOKENS,
        "treemap",
    );
    let mut ast = TreemapAst::default();

    let class_selector = |r: &mut Reader| -> Result<Option<String>, PelersError> {
        if r.accept(STYLE_SEPARATOR) {
            Ok(Some(r.expect(ID)?))
        } else {
            Ok(None)
        }
    };

    r.expect(KEYWORD)?;
    while r.kind() != -1 {
        let kind = r.kind();
        if kind == ACC_DESCR_KIND {
            ast.acc_descr = Some(acc_descr_value(&r.take()));
            continue;
        }
        if kind == ACC_TITLE_KIND {
            ast.acc_title = Some(acc_title_value(&r.take()));
            continue;
        }
        if kind == TITLE {
            ast.title = Some(title_value(&r.take()));
            continue;
        }
        if kind != INDENTATION && kind != STRING && kind != CLASS_DEF {
            return Err(r.fail("a node, a class definition, or a title"));
        }

        let indent = if kind == INDENTATION {
            Some(r.take().len())
        } else {
            None
        };
        if r.kind() == CLASS_DEF {
            let (class_name, style_text) = class_def_parts(&r.take());
            ast.rows.push(Row::ClassDef {
                indent,
                class_name,
                style_text: style_text.filter(|s| !s.is_empty()),
            });
            continue;
        }

        let quoted = r.expect(STRING)?;
        let name = quoted[1..quoted.len() - 1].to_owned();
        let after = if r.kind_at(r.i) == INDENTATION {
            r.i + 1
        } else {
            r.i
        };
        let is_leaf = matches!(r.kind_at(after), SEPARATOR | COMMA);
        let mut item = if is_leaf {
            r.i = after + 1;
            r.accept(INDENTATION);
            let value = js_parse_float(&r.expect(NUMBER)?.replace(',', ""));
            Item::Leaf {
                name,
                value,
                class_selector: None,
            }
        } else {
            Item::Section {
                name,
                class_selector: None,
            }
        };
        let selector = class_selector(&mut r)?;
        match &mut item {
            Item::Section { class_selector, .. } | Item::Leaf { class_selector, .. } => {
                *class_selector = selector;
            }
        }
        ast.rows.push(Row::Node { indent, item });
    }
    Ok(ast)
}
