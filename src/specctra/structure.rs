use super::common::ListToken;
use super::read::ReadDsn;
use super::read::{ListTokenizer, ParseError};
use super::write::ListWriter;
use super::write::WriteSes;
use specctra_derive::ReadDsn;
use specctra_derive::WriteSes;

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Dummy {}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct SesFile {
    pub session: Session,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Session {
    #[anon]
    pub id: String,
    pub routes: Routes,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Routes {
    pub resolution: Resolution,
    pub library_out: Library,
    pub network_out: NetworkOut,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct NetworkOut {
    #[vec("net")]
    pub net: Vec<NetOut>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct NetOut {
    #[anon]
    pub name: String,
    #[vec("wire")]
    pub wire: Vec<Wire>,
    #[vec("via")]
    pub via: Vec<Via>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct DsnFile {
    pub pcb: Pcb,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Pcb {
    #[anon]
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

#[derive(WriteSes, Debug)]
pub struct Parser {
    pub string_quote: Option<char>,
    pub space_in_quoted_tokens: Option<bool>,
    pub host_cad: Option<String>,
    pub host_version: Option<String>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Resolution {
    #[anon]
    pub unit: String,
    #[anon]
    pub value: f32,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Structure {
    #[vec("layer")]
    pub layers: Vec<Layer>,
    pub boundary: Boundary,
    #[vec("plane")]
    pub planes: Vec<Plane>,
    pub via: ViaNames,
    pub rule: Rule,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Layer {
    #[anon]
    pub name: String,
    pub r#type: String,
    pub property: Property,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Property {
    pub index: usize,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Boundary {
    pub path: Path,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Plane {
    #[anon]
    pub net: String,
    pub polygon: Polygon,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct ViaNames {
    #[anon_vec]
    pub names: Vec<String>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Placement {
    #[vec("component")]
    pub components: Vec<Component>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Component {
    #[anon]
    pub name: String,
    #[vec("place")]
    pub places: Vec<Place>,
}

#[derive(ReadDsn, WriteSes, Debug)]
#[allow(non_snake_case)]
pub struct Place {
    #[anon]
    pub name: String,
    #[anon]
    pub x: f32,
    #[anon]
    pub y: f32,
    #[anon]
    pub side: String,
    #[anon]
    pub rotation: f32,
    pub PN: Option<String>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Library {
    #[vec("image")]
    pub images: Vec<Image>,
    #[vec("padstack")]
    pub padstacks: Vec<Padstack>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Image {
    #[anon]
    pub name: String,
    #[vec("outline")]
    pub outlines: Vec<Outline>,
    #[vec("pin")]
    pub pins: Vec<Pin>,
    #[vec("keepout")]
    pub keepouts: Vec<Keepout>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Outline {
    pub path: Path,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Pin {
    #[anon]
    pub name: String,
    pub rotate: Option<f32>,
    #[anon]
    pub id: String,
    #[anon]
    pub x: f32,
    #[anon]
    pub y: f32,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Rotate {
    pub angle: f32,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Keepout {
    #[anon]
    pub idk: String,
    #[anon]
    pub shape: Shape,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Padstack {
    #[anon]
    pub name: String,
    #[vec("shape")]
    pub shapes: Vec<Shape>,
    pub attach: bool,
}

// TODO: derive for enums if more than this single one is needed
#[derive(Debug)]
pub enum Shape {
    Circle(Circle),
    Rect(Rect),
    Path(Path),
    Polygon(Polygon),
}

impl<R: std::io::BufRead> ReadDsn<R> for Shape {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        let name = tokenizer.consume_token()?.expect_any_start()?;
        let value = match name.as_str() {
            "circle" => Ok(Shape::Circle(tokenizer.read_value()?)),
            "rect" => Ok(Shape::Rect(tokenizer.read_value()?)),
            "path" => Ok(Shape::Path(tokenizer.read_value()?)),
            "polygon" => Ok(Shape::Polygon(tokenizer.read_value()?)),
            _ => Err(ParseError::Expected("a different keyword")),
        };
        tokenizer.consume_token()?.expect_end()?;
        value
    }
}

impl<W: std::io::Write> WriteSes<W> for Shape {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), std::io::Error> {
        match self {
            Self::Circle(inner) => writer.write_named("circle", inner),
            Self::Rect(inner) => writer.write_named("rect", inner),
            Self::Path(inner) => writer.write_named("path", inner),
            Self::Polygon(inner) => writer.write_named("polygon", inner),
        }
    }
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Circle {
    #[anon]
    pub layer: String,
    #[anon]
    pub diameter: u32,
    #[anon]
    pub offset: Option<Point>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Network {
    #[vec("net")]
    pub nets: Vec<NetPinAssignments>,
    #[vec("class")]
    pub classes: Vec<Class>,
}

#[derive(ReadDsn, WriteSes, Debug)]
// dsn names this "net", but it's a structure unrelated to "net" in wiring or elsewhere
pub struct NetPinAssignments {
    #[anon]
    pub name: String,
    pub pins: Pins,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Pins {
    #[anon_vec]
    pub names: Vec<String>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Class {
    #[anon]
    pub name: String,
    #[anon_vec]
    pub nets: Vec<String>,
    pub circuit: Circuit,
    pub rule: Rule,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Circuit {
    pub use_via: String,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Wiring {
    #[vec("wire")]
    pub wires: Vec<Wire>,
    #[vec("via")]
    pub vias: Vec<Via>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Wire {
    pub path: Path,
    pub net: String,
    pub r#type: String,
}

////////////////////////////////////////////
// structs that appear in multiple places //
////////////////////////////////////////////

// This type isn't meant to be deserialized as is (single points are
// more conveniently represented as fields on the enclosing struct)
// It exists to give a way to read arrays of coordinates
// (and enforce that such an array actually contains a whole number of points)
#[derive(Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

// Custom impl for the case described above
impl<R: std::io::BufRead> ReadDsn<R> for Vec<Point> {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        let mut array = Vec::<Point>::new();
        loop {
            let token = tokenizer.consume_token()?;
            if let ListToken::Leaf { value: ref x } = token {
                let x = x.parse::<f32>().unwrap();
                let y = tokenizer.read_value::<f32>()?;
                array.push(Point { x, y });
            } else {
                tokenizer.return_token(token);
                break;
            }
        }
        Ok(array)
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for Option<Point> {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        let token = tokenizer.consume_token()?;
        if let ListToken::Leaf { value: ref x } = token {
            let x = x.parse::<f32>().unwrap();
            let y = tokenizer.read_value::<f32>()?;
            Ok(Some(Point { x, y }))
        } else {
            tokenizer.return_token(token);
            Ok(None)
        }
    }
}

impl<W: std::io::Write> WriteSes<W> for Vec<Point> {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), std::io::Error> {
        for elem in self {
            writer.write_value(&elem.x)?;
            writer.write_value(&elem.y)?;
        }
        Ok(())
    }
}

impl<W: std::io::Write> WriteSes<W> for Option<Point> {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), std::io::Error> {
        if let Some(value) = self {
            writer.write_value(&value.x)?;
            writer.write_value(&value.y)?;
        }
        Ok(())
    }
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Polygon {
    #[anon]
    pub layer: String,
    #[anon]
    pub width: f32,
    #[anon]
    pub coords: Vec<Point>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Path {
    #[anon]
    pub layer: String,
    #[anon]
    pub width: f32,
    #[anon]
    pub coords: Vec<Point>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Rect {
    #[anon]
    pub layer: String,
    #[anon]
    pub x1: f32,
    #[anon]
    pub y1: f32,
    #[anon]
    pub x2: f32,
    #[anon]
    pub y2: f32,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Via {
    #[anon]
    pub name: String,
    #[anon]
    pub x: i32,
    #[anon]
    pub y: i32,
    pub net: String,
    pub r#type: String,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Rule {
    pub width: f32,
    #[vec("clearance")]
    pub clearances: Vec<Clearance>,
}

#[derive(ReadDsn, WriteSes, Debug)]
pub struct Clearance {
    #[anon]
    pub value: f32,
    pub r#type: Option<String>,
}
