use crate::template::base::{TemplateArgParser, TemplateArgProcesser};

pub const TEMPLATE_ARG_RING_PRESET: &str = "preset";

#[derive(Debug, Default)]
pub struct TemplateArgRingPresetParser;
impl TemplateArgRingPresetParser {
    pub fn parse(&self, arg: String) -> String {
        arg
    }
}

impl TemplateArgParser for TemplateArgRingPresetParser {
    fn name(&self) -> &str {
        TEMPLATE_ARG_RING_PRESET
    }
}

#[derive(Debug)]
pub struct TemplateArgRingPresetProcesser;
impl TemplateArgProcesser for TemplateArgRingPresetProcesser {
    fn process(&self, _: &str) -> Result<Box<dyn TemplateArgParser>, String> {
        Ok(Box::new(TemplateArgRingPresetParser))
    }
    fn name(&self) -> &str {
        TEMPLATE_ARG_RING_PRESET
    }
}
