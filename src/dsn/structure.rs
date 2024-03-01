use serde::{Deserialize, Deserializer, de::Error};

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
    pub plane: Option<Plane>,
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
#[serde(rename = "plane")]
pub struct Plane {
    net: String,
    shape: Polygon,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "via")]
pub struct Vias {
    vias: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "placement")]
pub struct Placement {
    pub components: Vec<Component>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "component")]
pub struct Component {
    pub name: String,
    pub places: Vec<Place>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "place")]
pub struct Place {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub side: String,
    pub rotation: f32,
    pub PN: Option<PN>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "PN")]
pub struct PN {
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "library")]
pub struct Library {
    pub images: Vec<Image>,
    pub padstacks: Vec<Padstack>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "image")]
pub struct Image {
    pub name: String,
    pub outlines: Vec<Outline>,
    pub pins: Vec<Pin>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "outline")]
pub struct Outline {
    pub path: Path,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "pin")]
pub struct Pin {
    pub name: String,
    pub rotate: Option<Rotate>,
    pub id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "rotate")]
pub struct Rotate {
    pub angle: f32,
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
    pub diameter: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "attach")]
pub struct Attach(pub bool);

#[derive(Deserialize, Debug)]
#[serde(rename = "network")]
pub struct Network {
    pub nets: Option<Vec<NetPinAssignments>>,
    pub classes: Vec<Class>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "net")]
// dsn names this "net", but it's a structure unrelated to "net" in wiring or elsewhere
pub struct NetPinAssignments {
    pub name: String,
    pub pins: Pins,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "pins")]
pub struct Pins {
    pub ids: Vec<String>,
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

#[derive(Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

fn de_points<'de, D>(deserializer: D) -> Result<Vec<Point>, D::Error>
    where D: Deserializer<'de>
{
    Vec::<f32>::deserialize(deserializer)?
        .chunks(2)
        .map(|pair| {
            let x = pair[0];
            let y = *pair.get(1).ok_or(
                Error::custom("expected paired x y coordinates, list ended at x")
            )?;

            Ok(Point { x, y })
        })
        .collect::<Result<Vec<Point>, D::Error>>()
}

#[derive(Deserialize, Debug)]
#[serde(rename = "polygon")]
pub struct Polygon {
    pub layer: String,
    pub width: f32,
    #[serde(deserialize_with = "de_points")]
    pub coords: Vec<Point>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "path")]
pub struct Path {
    pub layer: String,
    pub width: f32,
    #[serde(deserialize_with = "de_points")]
    pub coords: Vec<Point>,
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
