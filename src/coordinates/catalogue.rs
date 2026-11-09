//! Data only. Upstream definitions are community templates, not certification.

pub const SOURCE: &str = "https://github.com/geo-ts/msk";
pub const REVISION: &str = "946049d80f0f4b4956a53383961c8c6f4431169f";
pub const LICENSE: &str = include_str!("../../third_party/geo-ts-msk-LICENSE.txt");
const DATA: &str = include_str!("../../data/msk_templates.tsv");

#[derive(Debug, Clone, Copy)]
pub struct Template {
    pub id: &'static str,
    pub region: &'static str,
    pub name: &'static str,
    pub definition: &'static str,
}

pub fn all() -> impl Iterator<Item = Template> {
    DATA.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            Template {
                id: fields.next().expect("catalogue id"),
                region: fields.next().expect("catalogue region"),
                name: fields.next().expect("catalogue name"),
                definition: fields.next().expect("catalogue definition"),
            }
        })
}

pub fn for_region(region: &str) -> Vec<Template> {
    all().filter(|template| template.region == region).collect()
}
