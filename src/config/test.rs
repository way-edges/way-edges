use super::*;
#[allow(dead_code)]
pub fn get_config_test() {
    let res = get_config(&None).unwrap();
    println!("res: {res:#?}");
}

#[allow(dead_code)]
pub fn parse_config_test() {
    let data = r#"
    {
        "$schema": "sfa",
        "groups": [
            {
                "name": "test",
                "widgets": [{
                    "event_map": [[ 0, "ee" ]]
                }]
            }
        ]
    }
    "#;
    let res = parse::parse_config(data, &None).unwrap();
    println!("{res:#?}");
}
