use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename = "pcb")]
pub struct Pcb {
    pub name: String,
    pub parser: Parser,
    pub resolution: Resolution,
    pub unit: Option<Unit>,
    pub structure: Structure,
    pub placement: Placement,
    pub library: Library,
    pub network: Network,
    pub wiring: Wiring,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "parser")]
pub struct Parser {
    pub string_quote: Option<StringQuote>,
    pub space_in_quoted_tokens: SpaceAllowed,
    pub host_cad: Option<HostCad>,
    pub host_version: Option<HostVersion>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename = "string_quote")]
pub struct StringQuote(pub char);

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename = "space_in_quoted_tokens")]
pub struct SpaceAllowed(pub bool);

#[derive(Deserialize, Debug)]
#[serde(rename = "host_cad")]
pub struct HostCad(pub String);

#[derive(Deserialize, Debug)]
#[serde(rename = "host_version")]
pub struct HostVersion(pub String);

#[derive(Deserialize, Debug)]
#[serde(rename = "resolution")]
pub struct Resolution {
    pub unit: String,
    pub value: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "unit")]
pub struct Unit(pub String);

#[derive(Deserialize, Debug)]
#[serde(rename = "structure")]
pub struct Structure {
    pub layers: Vec<Layer>,
    pub boundary: Boundary,
    pub vias: Vias,
    pub rule: Rule,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "layer")]
pub struct Layer {
    pub name: String,
    pub r#type: Type,
    pub property: Property,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "property")]
pub struct Property(Index);

#[derive(Deserialize, Debug)]
#[serde(rename = "index")]
pub struct Index(pub u32);

#[derive(Deserialize, Debug)]
#[serde(rename = "boundary")]
pub struct Boundary(pub Path);

#[derive(Deserialize, Debug)]
#[serde(rename = "via")]
pub struct Vias {
    vias: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "placement")]
pub struct Placement;

#[derive(Deserialize, Debug)]
#[serde(rename = "library")]
pub struct Library {
    pub padstacks: Vec<Padstack>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "padstack")]
pub struct Padstack {
    pub name: String,
    pub shapes: Vec<Shape>,
    pub attach: Attach,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "shape")]
pub struct Shape(pub Circle);

#[derive(Deserialize, Debug)]
#[serde(rename = "circle")]
pub struct Circle {
    pub layer: String,
    pub radius: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "attach")]
pub struct Attach(pub bool);

#[derive(Deserialize, Debug)]
#[serde(rename = "network")]
pub struct Network {
    pub classes: Vec<Class>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "class")]
pub struct Class {
    pub name: String,
    pub nets: Vec<String>,
    pub circuit: Circuit,
    pub rule: Rule,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "circuit")]
pub struct Circuit(pub UseVia);

#[derive(Deserialize, Debug)]
#[serde(rename = "use_via")]
pub struct UseVia {
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "wiring")]
pub struct Wiring {
    pub wires: Vec<Wire>,
    pub vias: Vec<Via>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "wire")]
pub struct Wire {
    pub path: Path,
    pub net: Net,
    pub r#type: Type,
}

// structs that appear in multiple places

#[derive(Deserialize, Debug)]
#[serde(rename = "type")]
pub struct Type(pub String);

#[derive(Deserialize, Debug)]
#[serde(rename = "unit")]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug)]
#[serde(from = "FlatPath")]
pub struct Path {
    pub layer: String,
    pub width: u32,
    pub coords: Vec<Point>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "path")]
struct FlatPath {
    pub layer: String,
    pub width: u32,
    pub coords: Vec<i32>,
}

impl From<FlatPath> for Path {
    fn from(flat: FlatPath) -> Path {
        Path {
            layer: flat.layer,
            width: flat.width,
            coords: flat
                .coords
                .chunks(2)
                .map(|pair| Point {
                    x: pair[0],
                    // it's possible to return an error instead of panicking if this From were TryFrom,
                    // but I don't think serde will let us grab and inspect it elsewhere
                    // so annotating this with line/column information later might be difficult?
                    y: *pair.get(1).expect("unpaired coordinate in path"),
                })
                .collect(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename = "via")]
pub struct Via {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub net: Net,
    pub r#type: Type,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "rule")]
pub struct Rule {
    pub width: Width,
    pub clearances: Vec<Clearance>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "net")]
pub struct Net(pub String);

#[derive(Deserialize, Debug)]
#[serde(rename = "width")]
pub struct Width(pub f32);

#[derive(Deserialize, Debug)]
#[serde(rename = "clearance")]
pub struct Clearance {
    pub value: f32,
    pub r#type: Option<Type>,
}
