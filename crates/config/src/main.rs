use knus::Decode;

mod kdl;

#[derive(Debug, Clone, Decode)]
pub enum TopLevelConf {
    Btn(Btn),
}

#[derive(Debug, Clone, Decode)]
pub struct Btn {
    #[knus(flatten(child))]
    pub common: kdl::common::CommonConfig,
}

fn main() {
    let config = match knus::parse::<Vec<TopLevelConf>>("aaa", include_str!("../debug.kdl")) {
        Ok(config) => config,
        Err(e) => {
            println!("{:?}", miette::Report::new(e));
            std::process::exit(1);
        }
    };
    println!("{:#?}", config);

    println!("Hello, world!");
}
