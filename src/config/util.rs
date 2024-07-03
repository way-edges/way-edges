use super::conf::*;

use serde::Deserializer;
use std::str::FromStr;

pub fn transform_num_or_relative<'de, D>(d: D) -> Result<NumOrRelative, D::Error>
where
    D: Deserializer<'de>,
{
    struct F64OrRelativeVisitor;
    impl<'de> serde::de::Visitor<'de> for F64OrRelativeVisitor {
        type Value = NumOrRelative;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or a string")
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(NumOrRelative::Num(v as f64))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(NumOrRelative::Num(v as f64))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(NumOrRelative::Num(v))
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // just `unwrap`, it's ok
            let re = regex::Regex::new(r"^(\d+(\.\d+)?)%\s*(.*)$").unwrap();

            if let Some(captures) = re.captures(v) {
                let percentage_str = captures.get(1).map_or("", |m| m.as_str());
                let percentage = f64::from_str(percentage_str).map_err(E::custom)?;

                // // description
                // let description = captures
                //     .get(3)
                //     .map(|m| {
                //         let desc = m.as_str().trim();
                //         if desc.is_empty() {
                //             None
                //         } else {
                //             Some(desc.to_string())
                //         }
                //     })
                //     .flatten();

                Ok(NumOrRelative::Relative(percentage * 0.01))
            } else {
                Err(E::custom(
                    "Input does not match the expected format.".to_string(),
                ))
            }
        }
    }
    d.deserialize_any(F64OrRelativeVisitor)
}
