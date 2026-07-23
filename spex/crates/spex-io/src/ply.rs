use anyhow::{bail, Context, Result};
use spex_core::Point;
use std::io::{BufRead, BufReader, Read};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScalarType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Float32,
    Float64,
}

impl ScalarType {
    fn size(self) -> usize {
        match self {
            ScalarType::Int8 | ScalarType::UInt8 => 1,
            ScalarType::Int16 | ScalarType::UInt16 => 2,
            ScalarType::Int32 | ScalarType::UInt32 | ScalarType::Float32 => 4,
            ScalarType::Float64 => 8,
        }
    }

    fn parse(name: &str) -> Result<Self> {
        Ok(match name {
            "char" | "int8" => ScalarType::Int8,
            "uchar" | "uint8" => ScalarType::UInt8,
            "short" | "int16" => ScalarType::Int16,
            "ushort" | "uint16" => ScalarType::UInt16,
            "int" | "int32" => ScalarType::Int32,
            "uint" | "uint32" => ScalarType::UInt32,
            "float" | "float32" => ScalarType::Float32,
            "double" | "float64" => ScalarType::Float64,
            other => bail!("unknown ply scalar type '{other}'"),
        })
    }
}

enum Property {
    Scalar { name: String, ty: ScalarType },
    List { count_ty: ScalarType, item_ty: ScalarType },
}

struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}

enum Format {
    Ascii,
    BinaryLittleEndian,
}

/// Reads a PLY point cloud (ASCII or binary_little_endian). Only the `vertex`
/// element is extracted; other elements (e.g. `face`) are parsed structurally
/// and discarded so binary byte offsets stay aligned.
pub fn read<R: Read>(reader: R) -> Result<Vec<Point>> {
    let mut r = BufReader::new(reader);
    let (format, elements) = read_header(&mut r)?;

    let mut points = Vec::new();
    for elem in &elements {
        match format {
            Format::Ascii => read_ascii_element(&mut r, elem, &mut points)?,
            Format::BinaryLittleEndian => read_binary_element(&mut r, elem, &mut points)?,
        }
    }
    Ok(points)
}

fn read_header<R: BufRead>(r: &mut R) -> Result<(Format, Vec<Element>)> {
    let mut format = None;
    let mut elements: Vec<Element> = Vec::new();

    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line).context("reading ply header")?;
        if n == 0 {
            bail!("unexpected EOF in ply header");
        }
        let line = line.trim();

        if line.is_empty() || line == "ply" || line.starts_with("comment") || line.starts_with("obj_info") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("format ") {
            format = Some(match rest.split_whitespace().next().unwrap_or("") {
                "ascii" => Format::Ascii,
                "binary_little_endian" => Format::BinaryLittleEndian,
                "binary_big_endian" => bail!("binary_big_endian PLY files are not supported"),
                other => bail!("unknown ply format '{other}'"),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("element ") {
            let mut parts = rest.split_whitespace();
            let name = parts.next().context("element missing name")?.to_string();
            let count: usize = parts
                .next()
                .context("element missing count")?
                .parse()
                .context("element count is not a number")?;
            elements.push(Element {
                name,
                count,
                properties: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("property ") {
            let elem = elements.last_mut().context("property declared before any element")?;
            if let Some(list_rest) = rest.strip_prefix("list ") {
                let mut parts = list_rest.split_whitespace();
                let count_ty = ScalarType::parse(parts.next().context("list property missing count type")?)?;
                let item_ty = ScalarType::parse(parts.next().context("list property missing item type")?)?;
                elem.properties.push(Property::List { count_ty, item_ty });
            } else {
                let mut parts = rest.split_whitespace();
                let ty = ScalarType::parse(parts.next().context("property missing type")?)?;
                let name = parts.next().context("property missing name")?.to_string();
                elem.properties.push(Property::Scalar { name, ty });
            }
            continue;
        }
        if line == "end_header" {
            break;
        }
        // Unknown header directive: ignore rather than fail.
    }

    let format = format.context("ply file missing 'format' line")?;
    Ok((format, elements))
}

fn extract_vertex(x: Option<f64>, y: Option<f64>, z: Option<f64>, rgb: (Option<f64>, Option<f64>, Option<f64>)) -> Result<Point> {
    let position = [
        x.context("vertex missing x")?,
        y.context("vertex missing y")?,
        z.context("vertex missing z")?,
    ];
    let color = match rgb {
        (Some(r), Some(g), Some(b)) => [r as u8, g as u8, b as u8],
        _ => [200, 200, 200],
    };
    Ok(Point { position, color })
}

fn read_ascii_element<R: BufRead>(r: &mut R, elem: &Element, points: &mut Vec<Point>) -> Result<()> {
    for _ in 0..elem.count {
        let mut line = String::new();
        loop {
            line.clear();
            let n = r.read_line(&mut line).with_context(|| format!("reading {} data", elem.name))?;
            if n == 0 {
                bail!("unexpected EOF reading '{}' element data", elem.name);
            }
            if !line.trim().is_empty() {
                break;
            }
        }
        if elem.name != "vertex" {
            continue; // whole line already consumed; nothing else to do
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let mut idx = 0usize;
        let (mut x, mut y, mut z) = (None, None, None);
        let (mut cr, mut cg, mut cb) = (None, None, None);
        for prop in &elem.properties {
            match prop {
                Property::List { .. } => bail!("list properties on 'vertex' element are not supported"),
                Property::Scalar { name, .. } => {
                    let tok = tokens
                        .get(idx)
                        .with_context(|| format!("vertex line has fewer fields than declared properties: '{line}'"))?;
                    let v: f64 = tok.parse().with_context(|| format!("invalid numeric value '{tok}'"))?;
                    match name.as_str() {
                        "x" => x = Some(v),
                        "y" => y = Some(v),
                        "z" => z = Some(v),
                        "red" | "r" => cr = Some(v),
                        "green" | "g" => cg = Some(v),
                        "blue" | "b" => cb = Some(v),
                        _ => {}
                    }
                    idx += 1;
                }
            }
        }
        points.push(extract_vertex(x, y, z, (cr, cg, cb))?);
    }
    Ok(())
}

fn read_scalar<R: Read>(r: &mut R, ty: ScalarType) -> Result<f64> {
    let mut buf = [0u8; 8];
    let size = ty.size();
    r.read_exact(&mut buf[..size]).context("unexpected EOF reading binary ply data")?;
    Ok(match ty {
        ScalarType::Int8 => buf[0] as i8 as f64,
        ScalarType::UInt8 => buf[0] as f64,
        ScalarType::Int16 => i16::from_le_bytes([buf[0], buf[1]]) as f64,
        ScalarType::UInt16 => u16::from_le_bytes([buf[0], buf[1]]) as f64,
        ScalarType::Int32 => i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as f64,
        ScalarType::UInt32 => u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as f64,
        ScalarType::Float32 => f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as f64,
        ScalarType::Float64 => f64::from_le_bytes(buf),
    })
}

fn read_binary_element<R: Read>(r: &mut R, elem: &Element, points: &mut Vec<Point>) -> Result<()> {
    for _ in 0..elem.count {
        let (mut x, mut y, mut z) = (None, None, None);
        let (mut cr, mut cg, mut cb) = (None, None, None);
        for prop in &elem.properties {
            match prop {
                Property::Scalar { name, ty } => {
                    let v = read_scalar(r, *ty)?;
                    if elem.name == "vertex" {
                        match name.as_str() {
                            "x" => x = Some(v),
                            "y" => y = Some(v),
                            "z" => z = Some(v),
                            "red" | "r" => cr = Some(v),
                            "green" | "g" => cg = Some(v),
                            "blue" | "b" => cb = Some(v),
                            _ => {}
                        }
                    }
                }
                Property::List { count_ty, item_ty } => {
                    let count = read_scalar(r, *count_ty)? as usize;
                    for _ in 0..count {
                        read_scalar(r, *item_ty)?;
                    }
                }
            }
        }
        if elem.name == "vertex" {
            points.push(extract_vertex(x, y, z, (cr, cg, cb))?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_ascii_ply_with_color() {
        let data = "ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n1 2 3 255 0 0\n4 5 6 0 255 0\n";
        let pts = read(Cursor::new(data)).unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(pts[0].color, [255, 0, 0]);
        assert_eq!(pts[1].color, [0, 255, 0]);
    }

    #[test]
    fn parses_binary_le_ply_without_color() {
        let mut data = Vec::new();
        data.extend_from_slice(b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n");
        data.extend_from_slice(&1.5f32.to_le_bytes());
        data.extend_from_slice(&2.5f32.to_le_bytes());
        data.extend_from_slice(&3.5f32.to_le_bytes());
        let pts = read(Cursor::new(data)).unwrap();
        assert_eq!(pts.len(), 1);
        assert!((pts[0].position[0] - 1.5).abs() < 1e-6);
        assert_eq!(pts[0].color, [200, 200, 200]);
    }

    #[test]
    fn skips_face_element_in_binary() {
        let mut data = Vec::new();
        data.extend_from_slice(b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n");
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());
        data.push(3u8); // list count
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&2i32.to_le_bytes());
        let pts = read(Cursor::new(data)).unwrap();
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].position, [1.0, 2.0, 3.0]);
    }
}
