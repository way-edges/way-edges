use std::str::FromStr;

use crate::template::base::{TemplateArgParser, TemplateArgProcesser};

pub const TEMPLATE_ARG_FLOAT: &str = "float";

#[derive(Debug)]
pub struct TemplateArgFloatParser {
    precision: usize,
    multiply: Option<f64>,
}
impl Default for TemplateArgFloatParser {
    fn default() -> Self {
        Self {
            precision: 2,
            multiply: None,
        }
    }
}
impl TemplateArgFloatParser {
    pub fn parse(&self, mut progress: f64) -> String {
        if let Some(multiply) = self.multiply {
            progress *= multiply
        }
        format!("{:.precision$}", progress, precision = self.precision)
    }
}
impl FromStr for TemplateArgFloatParser {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut template = Self::default();

        let s = s.trim();
        if s.is_empty() {
            return Ok(template);
        }

        let mut precision = s;
        let mut multiply = None;

        if let Some((p, m)) = s.split_once(',') {
            let p = p.trim();
            let m = m.trim();

            precision = p;
            if !m.is_empty() {
                multiply = Some(m);
            }
        }

        if !precision.is_empty() {
            template.precision = precision.parse::<usize>().map_err(|e| e.to_string())?;
        }

        if let Some(multiply) = multiply {
            template.multiply = Some(multiply.parse::<f64>().map_err(|e| e.to_string())?);
        }

        Ok(template)
    }
}

impl TemplateArgParser for TemplateArgFloatParser {
    fn name(&self) -> &str {
        TEMPLATE_ARG_FLOAT
    }
}

#[derive(Debug)]
pub struct TemplateArgFloatProcesser;
impl TemplateArgProcesser for TemplateArgFloatProcesser {
    fn process(&self, param: &str) -> Result<Box<dyn TemplateArgParser>, String> {
        let a = TemplateArgFloatParser::from_str(param)?;
        Ok(Box::new(a))
    }
    fn name(&self) -> &str {
        TEMPLATE_ARG_FLOAT
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_float_template() {
        macro_rules! test {
            ($i:expr) => {
                let res = TemplateArgFloatParser::from_str($i).unwrap().parse(0.5125);
                assert_eq!(res, "0.51");
            };
        }
        test!(",");
        test!("");
        test!("2,");
        test!("2");
        test!(" 2,");
        test!(" 2");
        test!("2,1");
        test!("2, 1");
        test!(" 2, 1 ");
        test!(" , 1 ");
    }

    #[test]
    fn test_float_template_parse() {
        macro_rules! test {
            ($i:expr, $s:expr) => {
                let res = TemplateArgFloatParser::from_str($i).unwrap().parse(0.5125);
                assert_eq!(res, $s);
            };
        }
        test!("0", "1");
        test!("1", "0.5");
        test!("2", "0.51");
        test!("4", "0.5125");
        test!("10", "0.5125000000");
        test!(",0", "0.00");
        test!("3,2", "1.025");
        test!(",10", "5.12");
        test!(",100", "51.25");
    }
}
