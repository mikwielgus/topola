use serde::{de::Error, Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct DsnFile {
    pub pcb: Pcb,
}

#[derive(Deserialize, Debug)]
pub struct Pcb {
    pub name: String,
    pub parser: Parser,
    pub resolution: Resolution,
    pub unit: String,
    pub structure: Structure,
    pub placement: Placement,
    pub library: Library,
    pub network: Network,
    pub wiring: Wiring,
}

#[derive(Deserialize, Debug)]
pub struct Parser {
    pub string_quote: Option<char>,
    pub space_in_quoted_tokens: Option<bool>,
    pub host_cad: Option<String>,
    pub host_version: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Resolution {
    pub unit: String,
    pub value: u32,
}

#[derive(Deserialize, Debug)]
pub struct Structure {
    pub layer_vec: Vec<Layer>,
    pub boundary: Boundary,
    pub plane_vec: Vec<Plane>,
    pub via: ViaNames,
    pub rule: Rule,
}

#[derive(Deserialize, Debug)]
pub struct Layer {
    pub name: String,
    pub r#type: String,
    pub property: Property,
}

#[derive(Deserialize, Debug)]
pub struct Property {
    pub index: usize,
}

#[derive(Deserialize, Debug)]
pub struct Boundary {
    pub path: Path,
}

#[derive(Deserialize, Debug)]
pub struct Plane {
    pub net: String,
    pub polygon: Polygon,
}

#[derive(Deserialize, Debug)]
pub struct ViaNames {
    pub name_vec: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Placement {
    pub component_vec: Vec<Component>,
}

#[derive(Deserialize, Debug)]
pub struct Component {
    pub name: String,
    pub place_vec: Vec<Place>,
}

#[derive(Deserialize, Debug)]
pub struct Place {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub side: String,
    pub rotation: f32,
    pub PN: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Library {
    pub image_vec: Vec<Image>,
    pub padstack_vec: Vec<Padstack>,
}

#[derive(Deserialize, Debug)]
pub struct Image {
    pub name: String,
    pub outline_vec: Vec<Outline>,
    pub pin_vec: Vec<Pin>,
    pub keepout_vec: Vec<Keepout>,
}

#[derive(Deserialize, Debug)]
pub struct Outline {
    pub path: Path,
}

#[derive(Deserialize, Debug)]
pub struct Pin {
    pub name: String,
    pub rotate: Option<f32>,
    pub id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize, Debug)]
pub struct Keepout {
    pub idk: String,
    pub shape_anonymous: Shape,
}

#[derive(Deserialize, Debug)]
pub struct Rotate {
    pub angle: f32,
}

#[derive(Deserialize, Debug)]
pub struct Padstack {
    pub name: String,
    pub shape_vec: Vec<Shape>,
    pub attach: bool,
}

#[derive(Deserialize, Debug)]
pub enum Shape {
    #[serde(rename = "circle")]
    Circle(Circle),
    #[serde(rename = "rect")]
    Rect(Rect),
    #[serde(rename = "path")]
    Path(Path),
    #[serde(rename = "polygon")]
    Polygon(Polygon),
}

#[derive(Deserialize, Debug)]
pub struct Circle {
    pub layer: String,
    pub diameter: u32,
    #[serde(deserialize_with = "de_point_optional")]
    pub offset: Option<Point>,
}

#[derive(Deserialize, Debug)]
pub struct Network {
    pub net_vec: Vec<NetPinAssignments>,
    pub class_vec: Vec<Class>,
}

#[derive(Deserialize, Debug)]
// dsn names this "net", but it's a structure unrelated to "net" in wiring or elsewhere
pub struct NetPinAssignments {
    pub name: String,
    pub pins: Pins,
}

#[derive(Deserialize, Debug)]
pub struct Pins {
    pub names: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Class {
    pub name: String,
    pub net_vec: Vec<String>,
    pub circuit: Circuit,
    pub rule: Rule,
}

#[derive(Deserialize, Debug)]
pub struct Circuit {
    pub use_via: UseVia,
}

#[derive(Deserialize, Debug)]
pub struct UseVia {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Wiring {
    pub wire_vec: Vec<Wire>,
    pub via_vec: Vec<Via>,
}

#[derive(Deserialize, Debug)]
pub struct Wire {
    pub path: Path,
    pub net: String,
    pub r#type: String,
}

// structs that appear in multiple places

// This type isn't deserialized as is. Instead, Vec<Point> is converted from
// what's effectively Vec<f32> (with even length) in the file.
// Use #[serde(deserialize_with = "de_points")] for Vec<Point>
// and #[serde(deserialize_with = "de_point_optional")] for a single Point.

#[derive(Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

// Used to deserialize Option<Point>
fn de_point_optional<'de, D>(deserializer: D) -> Result<Option<Point>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut vec: Vec<Point> = Vec::<f32>::deserialize(deserializer)?
        .chunks(2)
        .map(|pair| {
            // 0th index is guaranteed to exist by `.chunks()`
            // (it ends iteration instead of emitting an empty Vec)
            let x = pair[0];
            // but if the file is malformed we may get an odd number of floats
            let y = *pair.get(1).ok_or(Error::custom(
                "expected paired x y coordinates, list ended at x",
            ))?;

            Ok(Point { x, y })
        })
        .collect::<Result<Vec<Point>, D::Error>>()?;

    if vec.len() > 1 {
        Err(Error::custom("expected a single pair of coordinates"))
    } else {
        Ok(vec.pop())
    }
}

// Used to deserialize Vec<Point>.
fn de_points<'de, D>(deserializer: D) -> Result<Vec<Point>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<f32>::deserialize(deserializer)?
        .chunks(2)
        .map(|pair| {
            // 0th index is guaranteed to exist by `.chunks()`
            // (it ends iteration instead of emitting an empty Vec)
            let x = pair[0];
            // but if the file is malformed we may get an odd number of floats
            let y = *pair.get(1).ok_or(Error::custom(
                "expected paired x y coordinates, list ended at x",
            ))?;

            Ok(Point { x, y })
        })
        .collect::<Result<Vec<Point>, D::Error>>()
}

#[derive(Deserialize, Debug)]
pub struct Polygon {
    pub layer: String,
    pub width: f32,
    #[serde(deserialize_with = "de_points")]
    pub coord_vec: Vec<Point>,
}

#[derive(Deserialize, Debug)]
pub struct Path {
    pub layer: String,
    pub width: f32,
    #[serde(deserialize_with = "de_points")]
    pub coord_vec: Vec<Point>,
}

#[derive(Deserialize, Debug)]
pub struct Rect {
    pub layer: String,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Deserialize, Debug)]
pub struct Via {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub net: String,
    pub r#type: String,
}

#[derive(Deserialize, Debug)]
pub struct Rule {
    pub width: f32,
    pub clearance_vec: Vec<Clearance>,
}

#[derive(Deserialize, Debug)]
pub struct Clearance {
    pub value: f32,
    pub r#type: Option<String>,
}
