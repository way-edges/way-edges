use std::{borrow::Cow, collections::HashMap, fmt::Debug};

use downcast_rs::Downcast;

use crate::notify_send;

pub trait TemplateArgParser: Debug + Downcast {
    fn name(&self) -> &str;
}
downcast_rs::impl_downcast!(TemplateArgParser);
pub trait TemplateArgProcesser: Debug {
    fn process(&self, param: &str) -> Result<Box<dyn TemplateArgParser>, String>;
    fn name(&self) -> &str;
}

pub struct TemplateProcesser(HashMap<String, Box<dyn TemplateArgProcesser>>);
impl TemplateProcesser {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn add_processer(mut self, p: impl TemplateArgProcesser + 'static) -> Self {
        self.0.insert(p.name().to_string(), Box::new(p));
        self
    }
    fn get(&self, name: &str) -> Option<&dyn TemplateArgProcesser> {
        self.0.get(name).map(|f| f.as_ref())
    }
}
impl Default for TemplateProcesser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum TemplateContent {
    String(String),
    Template(Box<dyn TemplateArgParser>),
}

#[derive(Debug)]
pub struct Template {
    pub contents: Vec<TemplateContent>,
}
impl Template {
    pub fn create_from_str(raw: &str, processers: TemplateProcesser) -> Result<Self, String> {
        let mut contents = vec![];
        let mut record_index = 0;

        let matches = extract_braces(raw)?;
        for m in matches {
            // push string before
            let end = m.start;
            if end > record_index {
                contents.push(TemplateContent::String(
                    raw[record_index..end].replace(r"\", ""),
                ))
            }
            record_index = m.end;

            // push template
            let template = m.get_content();
            let (name, arg) = match template.split_once(":") {
                Some((n, a)) => (n.trim(), a.trim()),
                None => (template.trim(), ""),
            };

            let Some(processer) = processers.get(name) else {
                let msg = format!("Unknown template: {name}");
                notify_send("Way-Eges", &msg, true);
                log::error!("{msg}");
                continue;
            };

            let processed = processer.process(arg);
            let Ok(parser) = processed else {
                let msg = format!("Faild to parse template: {name}: {processed:?}");
                notify_send("Way-Eges", &msg, true);
                log::error!("{msg}");
                continue;
            };

            contents.push(TemplateContent::Template(parser));
        }

        if record_index < raw.len() {
            contents.push(TemplateContent::String(
                raw[record_index..].replace(r"\", ""),
            ));
        };

        Ok(Self { contents })
    }

    pub fn parse(&mut self, mut cb: impl FnMut(&mut dyn TemplateArgParser) -> String) -> String {
        self.contents
            .iter_mut()
            .map(|content| match content {
                TemplateContent::String(s) => Cow::Borrowed(s.as_str()),
                TemplateContent::Template(parser) => Cow::Owned(cb(parser.as_mut())),
            })
            .collect::<Vec<Cow<str>>>()
            .join("")
            .to_string()
    }
}

struct BraceMatch<'a> {
    start: usize,
    end: usize,
    s: &'a str,
}
impl<'a> BraceMatch<'a> {
    fn from_total(start: usize, end: usize, total: &'a str) -> Self {
        let s = &total[start..end];
        Self { start, end, s }
    }
    fn get_content(&self) -> &str {
        &self.s[1..self.s.len() - 1]
    }
}

fn extract_braces(input: &str) -> Result<Vec<BraceMatch<'_>>, String> {
    let chars = input.chars().enumerate();

    struct BraceState<'a> {
        start: i32,
        indexes: Vec<BraceMatch<'a>>,
        str: &'a str,
    }
    impl<'a> BraceState<'a> {
        fn new(s: &'a str) -> Self {
            Self {
                start: -1,
                indexes: vec![],
                str: s,
            }
        }
        fn enter(&mut self, index: i32) {
            self.start = index;
        }
        fn leave(&mut self, index: i32) {
            if self.start != -1 {
                self.indexes.push(BraceMatch::from_total(
                    self.start as usize,
                    (index + 1) as usize,
                    self.str,
                ));
            }
            self.start = -1
        }
    }

    let mut state = BraceState::new(input);
    let mut escaped = false;

    for (index, c) in chars {
        match c {
            '\\' => {
                escaped = !escaped;
            }
            '{' if !escaped => {
                state.enter(index as i32);
            }
            '}' if !escaped => {
                state.leave(index as i32);
            }
            _ => {}
        }
    }
    let BraceState {
        start: _,
        indexes,
        str: _,
    } = state;

    Ok(indexes)
}

#[macro_export]
macro_rules! template_parser {
    ($visibility:vis, $name:ident, $s:expr) => {
        use paste::paste;
        paste! {
            $visibility const [<TEMPLATE_ARG_ $name:upper>]: &str = $s;
            impl TemplateArgParser for [<$name Parser>] {
                fn name(&self) -> &str {
                    $s
                }
            }
            #[derive(Debug)]
            $visibility struct [<$name Processer>];
            impl TemplateArgProcesser for [<$name Processer>] {
                fn process(&self, p: &str) -> Result<Box<dyn TemplateArgParser>, String> {
                    let parser = [<$name Parser>]::from_str(p)?;
                    Ok(Box::new(parser))
                }
                fn name(&self) -> &str {
                    $s
                }
            }

        }
    };
}

mod test {
    #![allow(unused_imports)]
    use super::*;
    use crate::template::arg::{
        TemplateArgFloatParser, TemplateArgFloatProcesser, TEMPLATE_ARG_FLOAT,
    };
    use std::str::FromStr;

    template_parser!(, Preset, "preset");
    #[derive(Debug)]
    pub struct PresetParser;
    impl FromStr for PresetParser {
        type Err = String;
        fn from_str(_: &str) -> Result<Self, Self::Err> {
            Ok(PresetParser)
        }
    }

    macro_rules! make_template {
        ($name:ident, $i:expr, $($pat:tt)*) => {
            let $($pat)* $name = Template::create_from_str(
                $i,
                TemplateProcesser::new()
                    .add_processer(TemplateArgFloatProcesser)
                    .add_processer(PresetProcesser),
            )
            .unwrap();
        };
    }

    #[test]
    fn test_ring_template() {
        macro_rules! test {
            ($i:expr, $s:expr) => {
                make_template!(temp, $i,);
                let len = temp.contents.len();
                assert_eq!(len, $s);
            };
        }
        // test!("{}", 0);
        test!(r"\{\}", 1);
        test!(r"\{}", 1);
        test!(r"{\}", 1);
        test!("{preset:}{float:}", 2);
        test!("{preset}{float}", 2);
        test!(" {preset}{float}", 3);
        test!("{preset} {float}", 3);
        test!("{preset}{float} ", 3);
        test!(" {preset} {float} ", 5);
        test!("  { preset }  { float }  ", 5);
        test!("{{preset}}{float}", 4);
        test!(r"\{preset\} \{float\}", 1);
        test!(r"\{preset\} {float\}", 1);
        test!("{{preset}}{{float}", 4);
    }

    #[test]
    fn test_parse_content() {
        macro_rules! test {
            ($i:expr, $preset_str:expr, $float:expr, $s:expr) => {
                make_template!(temp, $i, mut);
                let res = temp.parse(|parser| {
                    let a = match parser.name() {
                        TEMPLATE_ARG_PRESET => $preset_str.to_string(),
                        TEMPLATE_ARG_FLOAT => {
                            let a = parser.downcast_ref::<TemplateArgFloatParser>().unwrap();
                            a.parse($float).clone()
                        }
                        _ => unreachable!(),
                    };
                    a
                });
                assert_eq!(res, $s);
            };
        }
        test!("", "hh", 2., "");
        test!("a", "hh", 2., "a");
        test!("{}{}", "hh", 2., "");
        test!("{float}", "hh", 2., "2.00");
        test!(" { preset}a{ float }", "hh", 2., " hha2.00");
    }
}
